//! 备份包与平台账户无关：固定 Argon2id、Zstd 压缩和 AES-256-GCM 全包认证。
use crate::{CredentialVault, SecurityError};
use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine, engine::general_purpose::STANDARD};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use terminal_storage::{
    BackupCounts, BackupImportMode, BackupImportSummary, BackupSettingWrite, MAX_DATABASE_BYTES,
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"TMETABK1";
const HEADER_BYTES: usize = 8 + 16 + 12;
const MAX_METADATA_BYTES: usize = 16 * 1024;
pub const MAX_PACKAGE_BYTES: usize = 192 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupMetadata {
    pub backup_id: String,
    pub created_at: u64,
    pub device_name: String,
    pub app_version: String,
    pub counts: BackupCounts,
}
pub struct OpenedBackup {
    pub metadata: BackupMetadata,
    pub database: Zeroizing<Vec<u8>>,
}
fn invalid() -> SecurityError {
    SecurityError::Vault("备份包无效、格式不兼容或超过资源限额")
}
fn password_valid(password: &str) -> Result<(), SecurityError> {
    if password.chars().count() < 12 || password.len() > 1024 {
        return Err(SecurityError::Vault(
            "备份密码须至少 12 个字符且不超过 1024 UTF-8 字节",
        ));
    }
    Ok(())
}
fn derive(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, SecurityError> {
    password_valid(password)?;
    let params = Params::new(65536, 3, 1, Some(32)).map_err(|_| invalid())?;
    let mut key = Zeroizing::new([0; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|_| invalid())?;
    Ok(key)
}
fn validate_metadata(metadata: &BackupMetadata) -> Result<(), SecurityError> {
    if uuid::Uuid::parse_str(&metadata.backup_id).is_err()
        || metadata.created_at == 0
        || metadata.device_name.trim().is_empty()
        || metadata.device_name.len() > 256
        || metadata.device_name.chars().any(char::is_control)
        || metadata.app_version.is_empty()
        || metadata.app_version.len() > 128
        || metadata.app_version.chars().any(char::is_control)
        || [
            metadata.counts.hosts,
            metadata.counts.groups,
            metadata.counts.credentials,
            metadata.counts.forwards,
            metadata.counts.known_hosts,
            metadata.counts.snippets,
        ]
        .into_iter()
        .try_fold(0u64, |sum, count| sum.checked_add(count))
        .is_none_or(|sum| sum > 100_000)
    {
        return Err(invalid());
    }
    Ok(())
}

/// 密码派生和压缩为阻塞计算，调用方须使用阻塞工作池。
pub fn seal(
    password: &str,
    metadata: &BackupMetadata,
    database: &[u8],
) -> Result<String, SecurityError> {
    password_valid(password)?;
    validate_metadata(metadata)?;
    if database.len() > MAX_DATABASE_BYTES || !database.starts_with(b"SQLite format 3\0") {
        return Err(invalid());
    }
    let metadata = Zeroizing::new(serde_json::to_vec(metadata).map_err(|_| invalid())?);
    if metadata.len() > MAX_METADATA_BYTES {
        return Err(invalid());
    }
    // 先压缩整个 SQLite 数据，再把元数据与压缩正文一起加密；外层不暴露设备或备份时间。
    let mut compressed = Zeroizing::new(Vec::new());
    {
        let mut encoder =
            zstd::stream::write::Encoder::new(&mut *compressed, 3).map_err(|_| invalid())?;
        encoder.write_all(database).map_err(|_| invalid())?;
        encoder.finish().map_err(|_| invalid())?;
    }
    let mut clear = Zeroizing::new(Vec::with_capacity(12 + metadata.len() + compressed.len()));
    clear.extend_from_slice(&(metadata.len() as u32).to_be_bytes());
    clear.extend_from_slice(&metadata);
    clear.extend_from_slice(&(database.len() as u64).to_be_bytes());
    clear.extend_from_slice(&compressed);
    let mut header = [0u8; HEADER_BYTES];
    header[..8].copy_from_slice(MAGIC);
    OsRng
        .try_fill_bytes(&mut header[8..])
        .map_err(|_| invalid())?;
    let key = derive(password, &header[8..24])?;
    let cipher = Aes256Gcm::new_from_slice(&*key).map_err(|_| invalid())?;
    let nonce: &[u8; 12] = header[24..36].try_into().map_err(|_| invalid())?;
    let ciphertext = cipher
        .encrypt(
            nonce.into(),
            Payload {
                msg: &clear,
                aad: &header,
            },
        )
        .map_err(|_| invalid())?;
    let mut binary = Vec::with_capacity(HEADER_BYTES + ciphertext.len());
    binary.extend_from_slice(&header);
    binary.extend_from_slice(&ciphertext);
    let encoded = STANDARD.encode(&binary);
    if encoded.len() > MAX_PACKAGE_BYTES {
        return Err(invalid());
    }
    Ok(encoded)
}

pub fn open(password: &str, package: &str) -> Result<OpenedBackup, SecurityError> {
    password_valid(password)?;
    if package.len() > MAX_PACKAGE_BYTES || package.len() < 80 {
        return Err(invalid());
    }
    let bytes = STANDARD.decode(package).map_err(|_| invalid())?;
    if bytes.len() <= HEADER_BYTES + 16 || &bytes[..8] != MAGIC {
        return Err(invalid());
    }
    let header = &bytes[..HEADER_BYTES];
    let key = derive(password, &header[8..24])?;
    let cipher = Aes256Gcm::new_from_slice(&*key).map_err(|_| invalid())?;
    let nonce: &[u8; 12] = header[24..36].try_into().map_err(|_| invalid())?;
    let clear = Zeroizing::new(
        cipher
            .decrypt(
                nonce.into(),
                Payload {
                    msg: &bytes[HEADER_BYTES..],
                    aad: header,
                },
            )
            .map_err(|_| SecurityError::Vault("备份密码不正确或备份包已损坏"))?,
    );
    if clear.len() < 4 {
        return Err(invalid());
    }
    let size = u32::from_be_bytes(clear[..4].try_into().map_err(|_| invalid())?) as usize;
    if size > MAX_METADATA_BYTES || clear.len() <= 12 + size {
        return Err(invalid());
    }
    let metadata: BackupMetadata =
        serde_json::from_slice(&clear[4..4 + size]).map_err(|_| invalid())?;
    validate_metadata(&metadata)?;
    let expected_size = u64::from_be_bytes(
        clear[4 + size..12 + size]
            .try_into()
            .map_err(|_| invalid())?,
    );
    if expected_size < 100 || expected_size > MAX_DATABASE_BYTES as u64 {
        return Err(invalid());
    }
    let mut decoder =
        zstd::stream::read::Decoder::new(&clear[12 + size..]).map_err(|_| invalid())?;
    // 限制帧窗口与解压输出，认证成功的包仍不获得无限内存预算。
    decoder.window_log_max(27).map_err(|_| invalid())?;
    let mut database = Zeroizing::new(Vec::with_capacity(expected_size as usize + 1));
    decoder
        .take(expected_size + 1)
        .read_to_end(&mut database)
        .map_err(|_| invalid())?;
    if database.len() != expected_size as usize || !database.starts_with(b"SQLite format 3\0") {
        return Err(invalid());
    }
    let counts = terminal_storage::inspect_backup_database(database.to_vec())?;
    if counts != metadata.counts {
        return Err(SecurityError::Vault("备份元数据与数据库内容不一致"));
    }
    Ok(OpenedBackup { metadata, database })
}
impl CredentialVault {
    /// 与凭据多步写入和改密共用门闩；网络与密码派生必须在返回后进行。
    pub async fn export_backup_database(&self) -> Result<Zeroizing<Vec<u8>>, SecurityError> {
        let _operation = self.operations.lock().await;
        if !self.is_unlocked() {
            return Err(SecurityError::NotReady);
        }
        Ok(self.store.export_backup_database().await?)
    }
    pub async fn import_backup_database(
        &self,
        database: Vec<u8>,
        mode: BackupImportMode,
    ) -> Result<BackupImportSummary, SecurityError> {
        self.import_backup_database_with_setting(database, mode, None)
            .await
    }
    pub async fn import_backup_database_with_setting(
        &self,
        database: Vec<u8>,
        mode: BackupImportMode,
        setting: Option<BackupSettingWrite>,
    ) -> Result<BackupImportSummary, SecurityError> {
        let mut database = Zeroizing::new(database);
        let _operation = self.operations.lock().await;
        if !self.is_unlocked() {
            return Err(SecurityError::NotReady);
        }
        Ok(self
            .store
            .import_backup_database_with_setting(std::mem::take(&mut *database), mode, setting)
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(flavor = "current_thread")]
    async fn package_hides_metadata_and_rejects_wrong_password_tampering_and_count_mismatch() {
        let path = std::env::temp_dir().join(format!(
            "termeta-backup-crypto-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&path).unwrap();
        let store = terminal_storage::SqliteStore::locked(path.join("source.sqlite3"));
        store.unlock(&[17; 32]).await.unwrap();
        let database = store.export_backup_database().await.unwrap();
        let metadata = BackupMetadata {
            backup_id: uuid::Uuid::new_v4().to_string(),
            created_at: 1_792_186_000_000,
            device_name: "秘密设备名称".into(),
            app_version: "0.1.0".into(),
            counts: BackupCounts::default(),
        };
        let package = seal("a-long-backup-password", &metadata, &database).unwrap();
        let second = seal("a-long-backup-password", &metadata, &database).unwrap();
        assert_ne!(package, second);
        assert!(package.len() < database.len());
        let binary = STANDARD.decode(&package).unwrap();
        assert!(
            !binary
                .windows(metadata.device_name.len())
                .any(|bytes| bytes == metadata.device_name.as_bytes())
        );
        let opened = open("a-long-backup-password", &package).unwrap();
        assert_eq!(opened.metadata.device_name, metadata.device_name);
        assert_eq!(opened.database.as_slice(), database.as_slice());
        assert!(open("wrong-backup-password", &package).is_err());
        let mut tampered = binary;
        *tampered.last_mut().unwrap() ^= 1;
        assert!(open("a-long-backup-password", &STANDARD.encode(tampered)).is_err());
        let mut mismatch = metadata;
        mismatch.counts.hosts = 1;
        let invalid = seal("a-long-backup-password", &mismatch, &database).unwrap();
        assert!(open("a-long-backup-password", &invalid).is_err());
        drop(store);
        let _ = std::fs::remove_dir_all(path);
    }
}

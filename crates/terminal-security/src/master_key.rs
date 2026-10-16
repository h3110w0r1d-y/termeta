use crate::{CredentialVault, SecurityError};
use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

const SERVICE: &str = "com.h3110w0r1d.termvexa";
const ACCOUNT: &str = "master-key-v2";
const KDF: &str = "argon2id-v19-m65536-t3-p1";
const CHECK: &[u8] = b"termvexa/encrypted-vault/v2";

/// 平台适配只持有一个主密钥；可注入测试适配器，避免测试访问真实钥匙串。
pub(crate) trait MasterKeyBackend: Send + Sync {
    fn load(&self) -> Result<Zeroizing<Vec<u8>>, SecurityError>;
    fn save(&self, key: &[u8]) -> Result<(), SecurityError>;
}
pub(crate) struct OsMasterKey;
impl MasterKeyBackend for OsMasterKey {
    fn load(&self) -> Result<Zeroizing<Vec<u8>>, SecurityError> {
        keyring::Entry::new(SERVICE, ACCOUNT)
            .and_then(|entry| entry.get_secret())
            .map(Zeroizing::new)
            .map_err(|_| SecurityError::Unavailable)
    }
    fn save(&self, key: &[u8]) -> Result<(), SecurityError> {
        keyring::Entry::new(SERVICE, ACCOUNT)
            .and_then(|entry| entry.set_secret(key))
            .map_err(|_| SecurityError::Unavailable)
    }
}

#[derive(Default)]
pub(crate) struct BootState {
    attempted: bool,
    warning: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub mode: &'static str,
    pub keychain_warning: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    format: u32,
    id: String,
    kdf: String,
    salt: [u8; 16],
    nonce: [u8; 12],
    verifier: Vec<u8>,
    initialized: bool,
}
fn unavailable() -> SecurityError {
    SecurityError::Vault("保险库元数据不可用或已损坏，请保留文件后重试")
}
fn derive(password: &str, salt: &[u8; 16]) -> Result<Zeroizing<[u8; 32]>, SecurityError> {
    let params = Params::new(65536, 3, 1, Some(32)).map_err(|_| unavailable())?;
    let mut key = Zeroizing::new([0; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|_| unavailable())?;
    Ok(key)
}
impl Header {
    fn aad(&self) -> Vec<u8> {
        let mut aad = [CHECK, self.id.as_bytes(), self.kdf.as_bytes(), &self.salt].concat();
        aad.push(u8::from(self.initialized));
        aad
    }
    fn verify(&self, key: &[u8; 32]) -> Result<(), SecurityError> {
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| unavailable())?;
        let clear = cipher
            .decrypt(
                (&self.nonce).into(),
                Payload {
                    msg: &self.verifier,
                    aad: &self.aad(),
                },
            )
            .map_err(|_| SecurityError::Vault("密码不正确或保险库校验数据损坏"))?;
        if clear != CHECK {
            return Err(unavailable());
        }
        Ok(())
    }
    fn seal(&mut self, key: &[u8; 32]) -> Result<(), SecurityError> {
        // 完成标记也是认证数据；状态变更必须使用新的 nonce，禁止重用同一 GCM nonce。
        OsRng
            .try_fill_bytes(&mut self.nonce)
            .map_err(|_| unavailable())?;
        self.verifier = Aes256Gcm::new_from_slice(key)
            .map_err(|_| unavailable())?
            .encrypt(
                (&self.nonce).into(),
                Payload {
                    msg: CHECK,
                    aad: &self.aad(),
                },
            )
            .map_err(|_| unavailable())?;
        Ok(())
    }
    fn create(password: &str) -> Result<(Self, Zeroizing<[u8; 32]>), SecurityError> {
        let mut header = Self {
            format: 2,
            id: uuid::Uuid::new_v4().to_string(),
            kdf: KDF.into(),
            salt: [0; 16],
            nonce: [0; 12],
            verifier: Vec::new(),
            initialized: false,
        };
        OsRng
            .try_fill_bytes(&mut header.salt)
            .map_err(|_| unavailable())?;
        let key = derive(password, &header.salt)?;
        header.seal(&key)?;
        Ok((header, key))
    }
}
fn read_header(path: &Path) -> Result<Option<Header>, SecurityError> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(unavailable()),
    };
    let mut bytes = Vec::new();
    file.take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    if bytes.len() > 4096 {
        return Err(unavailable());
    }
    let header: Header = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
    if header.format != 2
        || header.kdf != KDF
        || uuid::Uuid::parse_str(&header.id).is_err()
        || header.verifier.len() != CHECK.len() + 16
    {
        return Err(unavailable());
    }
    Ok(Some(header))
}
fn write_header(path: &Path, header: &Header, replace: bool) -> Result<(), SecurityError> {
    let target = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&target).map_err(|_| unavailable())?;
        file.write_all(&serde_json::to_vec(header).map_err(|_| unavailable())?)
            .map_err(|_| unavailable())?;
        file.sync_all().map_err(|_| unavailable())?;
        if replace {
            std::fs::rename(&target, path).map_err(|_| unavailable())?;
        } else {
            // 首次元数据也原子发布，避免中断写入遗失盐；独占链接禁止覆盖已有保险库。
            std::fs::hard_link(&target, path).map_err(|_| unavailable())?;
            std::fs::remove_file(&target).map_err(|_| unavailable())?;
        }
        #[cfg(unix)]
        if let Some(parent) = path.parent() {
            std::fs::File::open(parent)
                .and_then(|dir| dir.sync_all())
                .map_err(|_| unavailable())?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(target);
    }
    result
}
impl CredentialVault {
    fn header_path(&self) -> PathBuf {
        self.store.path().with_extension("vault.json")
    }
    fn status_value(&self, boot: &BootState, exists: bool) -> VaultStatus {
        VaultStatus {
            mode: if self.is_unlocked() {
                "unlocked"
            } else if exists {
                "locked"
            } else {
                "setup"
            },
            keychain_warning: boot.warning.clone(),
        }
    }
    pub fn is_unlocked(&self) -> bool {
        self.unlocked.load(std::sync::atomic::Ordering::Acquire)
    }
    /// 每个进程最多自动读取一次主密钥，拒绝授权后由用户通过密码恢复。
    pub async fn status(&self) -> Result<VaultStatus, SecurityError> {
        let mut boot = self.boot.lock().await;
        let header = read_header(&self.header_path())?;
        if let Some(header) = &header {
            if !self.is_unlocked() && !boot.attempted {
                boot.attempted = true;
                let backend = self.key_backend.clone();
                let key = tokio::task::spawn_blocking(move || backend.load())
                    .await
                    .map_err(|_| unavailable())?;
                let result = match key {
                    Ok(key) => match <&[u8; 32]>::try_from(key.as_slice()) {
                        Ok(key) => match header.verify(key) {
                            Ok(()) => self.open_database(header, key).await,
                            Err(error) => Err(error),
                        },
                        Err(_) => Err(unavailable()),
                    },
                    Err(_) => Err(SecurityError::Unavailable),
                };
                if result.is_err() {
                    boot.warning = Some("未能使用系统钥匙串自动解锁，请输入保险库密码。".into());
                }
            }
        } else if self.store.path().exists() {
            return Err(SecurityError::Vault(
                "数据库存在但解锁元数据缺失，请恢复对应的 vault.json 文件；不会重建或覆盖数据库",
            ));
        }
        Ok(self.status_value(&boot, header.is_some()))
    }
    async fn open_database(&self, header: &Header, key: &[u8; 32]) -> Result<(), SecurityError> {
        if header.initialized {
            let length = std::fs::metadata(self.store.path())
                .map_err(|_| {
                    SecurityError::Vault("加密数据库文件缺失，请恢复原数据库；不会自动创建空库")
                })?
                .len();
            if length < 4096 {
                return Err(SecurityError::Vault(
                    "加密数据库已截断，无法解密；不会重建空库",
                ));
            }
        }
        self.store.unlock(key).await?;
        if !header.initialized {
            let mut complete = header.clone();
            complete.initialized = true;
            complete.seal(key)?;
            write_header(&self.header_path(), &complete, true)?;
        }
        self.unlocked
            .store(true, std::sync::atomic::Ordering::Release);
        let _ = self.reconcile_deletions().await;
        Ok(())
    }
    /// 首次设置或密码恢复均在 Rust 内派生密钥；密码、密钥不进入返回 DTO。
    pub async fn unlock_with_password(
        &self,
        password: Zeroizing<String>,
        setup: bool,
    ) -> Result<VaultStatus, SecurityError> {
        let mut boot = self.boot.lock().await;
        if self.is_unlocked() {
            return Ok(self.status_value(&boot, true));
        }
        if password.len() > 1024 || password.is_empty() || (setup && password.chars().count() < 12)
        {
            return Err(SecurityError::Vault(
                "保险库密码至少12个字符，且不超过1024字节",
            ));
        }
        let path = self.header_path();
        let existing = read_header(&path)?;
        if setup != existing.is_none() {
            return Err(SecurityError::Vault("保险库状态已变化，请重新加载"));
        }
        if setup && self.store.path().exists() {
            return Err(unavailable());
        }
        let (header, key) = tokio::task::spawn_blocking(move || {
            if let Some(header) = existing {
                let key = derive(&password, &header.salt)?;
                header.verify(&key)?;
                Ok((header, key))
            } else {
                Header::create(&password)
            }
        })
        .await
        .map_err(|_| unavailable())??;
        // 先持久化盐和校验记录；初始化中断仍能凭密码重试，不能丢失唯一的派生参数。
        if setup {
            write_header(&path, &header, false)?;
        }
        self.open_database(&header, &key).await?;
        boot.attempted = true;
        let backend = self.key_backend.clone();
        let remembered = tokio::task::spawn_blocking(move || backend.save(key.as_ref()))
            .await
            .unwrap_or(Err(SecurityError::Unavailable));
        boot.warning = remembered.err().map(|_| {
            "本次已解锁，但主密钥未能保存到系统钥匙串；下次启动需再次输入保险库密码。".into()
        });
        Ok(self.status_value(&boot, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn password_recovery_rejects_wrong_password_and_tampering() {
        let (header, key) = Header::create("正确的保险库密码-test-123").unwrap();
        header.verify(&key).unwrap();
        let recovered = derive("正确的保险库密码-test-123", &header.salt).unwrap();
        assert_eq!(&*key, &*recovered);
        let wrong = derive("错误密码-test-456", &header.salt).unwrap();
        assert!(header.verify(&wrong).is_err());
        let mut changed = header.clone();
        changed.salt[0] ^= 1;
        assert!(changed.verify(&key).is_err());
        let mut changed = header.clone();
        changed.verifier[0] ^= 1;
        assert!(changed.verify(&key).is_err());
        let mut changed = header.clone();
        changed.initialized = true;
        assert!(changed.verify(&key).is_err());
        let (other, _) = Header::create("正确的保险库密码-test-123").unwrap();
        assert_ne!(header.salt, other.salt);
    }

    #[derive(Default)]
    struct FakeKeychain {
        key: std::sync::Mutex<Option<Vec<u8>>>,
        reads: std::sync::atomic::AtomicUsize,
        writes: std::sync::atomic::AtomicUsize,
        fail_write: std::sync::atomic::AtomicBool,
    }
    impl MasterKeyBackend for FakeKeychain {
        fn load(&self) -> Result<Zeroizing<Vec<u8>>, SecurityError> {
            self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.key
                .lock()
                .unwrap()
                .clone()
                .map(Zeroizing::new)
                .ok_or(SecurityError::Unavailable)
        }
        fn save(&self, key: &[u8]) -> Result<(), SecurityError> {
            self.writes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.fail_write.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(SecurityError::Unavailable);
            }
            *self.key.lock().unwrap() = Some(key.to_vec());
            Ok(())
        }
    }
    struct TestPath(PathBuf);
    impl TestPath {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("termvexa-vault-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn vault(&self, backend: std::sync::Arc<FakeKeychain>) -> CredentialVault {
            let mut vault = CredentialVault::new(std::sync::Arc::new(
                terminal_storage::SqliteStore::locked(self.0.join("vault.sqlite3")),
            ));
            vault.key_backend = backend;
            vault
        }
    }
    impl Drop for TestPath {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[tokio::test(flavor = "current_thread")]
    async fn one_keychain_item_and_password_recovery() {
        use std::sync::{Arc, atomic::Ordering};
        let path = TestPath::new();
        let backend = Arc::new(FakeKeychain::default());
        let first = path.vault(backend.clone());
        assert_eq!(first.status().await.unwrap().mode, "setup");
        assert_eq!(
            first
                .unlock_with_password(Zeroizing::new("test-master-password".into()), true)
                .await
                .unwrap()
                .mode,
            "unlocked"
        );
        let credential = first
            .save_password("first", Zeroizing::new("secret".into()))
            .await
            .unwrap();
        first.load(&credential).await.unwrap();
        assert_eq!(backend.reads.load(Ordering::SeqCst), 0);
        assert_eq!(backend.writes.load(Ordering::SeqCst), 1);
        let second = path.vault(backend.clone());
        assert_eq!(second.status().await.unwrap().mode, "unlocked");
        second.status().await.unwrap();
        assert_eq!(backend.reads.load(Ordering::SeqCst), 1);
        *backend.key.lock().unwrap() = Some(vec![99; 32]);
        let third = path.vault(backend.clone());
        assert_eq!(third.status().await.unwrap().mode, "locked");
        third.status().await.unwrap();
        assert_eq!(backend.reads.load(Ordering::SeqCst), 2);
        assert!(
            third
                .unlock_with_password(Zeroizing::new("wrong-password".into()), false)
                .await
                .is_err()
        );
        assert!(!third.is_unlocked());
        assert_eq!(backend.writes.load(Ordering::SeqCst), 1);
        assert_eq!(
            third
                .unlock_with_password(Zeroizing::new("test-master-password".into()), false)
                .await
                .unwrap()
                .mode,
            "unlocked"
        );
        assert_eq!(third.load(&credential).await.unwrap().as_str(), "secret");
        assert_eq!(backend.writes.load(Ordering::SeqCst), 2);
    }
    #[tokio::test(flavor = "current_thread")]
    async fn keychain_write_failure_does_not_lose_password_recovery() {
        use std::sync::{Arc, atomic::Ordering};
        let path = TestPath::new();
        let backend = Arc::new(FakeKeychain::default());
        backend.fail_write.store(true, Ordering::SeqCst);
        let first = path.vault(backend.clone());
        let status = first
            .unlock_with_password(Zeroizing::new("test-master-password".into()), true)
            .await
            .unwrap();
        assert_eq!(status.mode, "unlocked");
        assert!(status.keychain_warning.is_some());
        let next = path.vault(backend.clone());
        assert_eq!(next.status().await.unwrap().mode, "locked");
        assert_eq!(
            next.unlock_with_password(Zeroizing::new("test-master-password".into()), false)
                .await
                .unwrap()
                .mode,
            "unlocked"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn interrupted_initialization_recovers_but_truncated_database_does_not_reset() {
        let path = TestPath::new();
        let backend = std::sync::Arc::new(FakeKeychain::default());
        let vault = path.vault(backend.clone());
        let (header, key) = Header::create("test-master-password").unwrap();
        write_header(&vault.header_path(), &header, false).unwrap();
        assert_eq!(vault.status().await.unwrap().mode, "locked");
        vault
            .unlock_with_password(Zeroizing::new("test-master-password".into()), false)
            .await
            .unwrap();
        let completed = read_header(&vault.header_path()).unwrap().unwrap();
        assert!(completed.initialized);
        assert_ne!(completed.nonce, header.nonce);
        completed.verify(&key).unwrap();
        // 用独立缺失数据库场景验证完成态保护，不截断仍由活动池打开的文件。
        let missing = TestPath::new();
        let next = missing.vault(backend);
        write_header(&next.header_path(), &completed, false).unwrap();
        std::fs::write(next.store.path(), []).unwrap();
        assert_eq!(next.status().await.unwrap().mode, "locked");
        assert!(
            next.unlock_with_password(Zeroizing::new("test-master-password".into()), false)
                .await
                .is_err()
        );
        assert!(!next.is_unlocked());
        assert_eq!(std::fs::metadata(next.store.path()).unwrap().len(), 0);
    }
}

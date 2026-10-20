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

/// 仅在 Rust 内存持有；不得进入 WebView、云配置或日志。
#[derive(Clone)]
pub struct VaultKey {
    pub key: Zeroizing<[u8; 32]>,
    pub salt: [u8; 16],
}

const SERVICE: &str = "io.termeta";
const ACCOUNT: &str = "master-key-v2";
const KDF: &str = "argon2id-v19-m65536-t3-p1";
const CHECK: &[u8] = b"termeta/encrypted-vault/v2";

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
    cleanup_warning: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub mode: &'static str,
    pub keychain_warning: Option<String>,
    pub cleanup_warning: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordChangeResult {
    pub keychain_warning: Option<String>,
    pub cleanup_warning: Option<String>,
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
fn cleanup_warning(pending: bool) -> Option<String> {
    pending.then(|| {
        "保险库恢复文件尚未完全清理；退出后重新启动将重试清理。旧副本仍受原密码保护。".into()
    })
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
            cleanup_warning: boot.cleanup_warning.clone(),
        }
    }
    pub fn is_unlocked(&self) -> bool {
        self.unlocked.load(std::sync::atomic::Ordering::Acquire) && self.store.is_unlocked()
    }
    /// 每个进程最多自动读取一次主密钥，拒绝授权后由用户通过密码恢复。
    pub async fn status(&self) -> Result<VaultStatus, SecurityError> {
        let mut boot = self.boot.lock().await;
        self.store.recover_password_change().await?;
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
                match result {
                    Ok(pending) => boot.cleanup_warning = cleanup_warning(pending),
                    Err(_) => {
                        boot.warning = Some("未能使用系统钥匙串自动解锁，请输入保险库密码。".into())
                    }
                }
            }
        } else if self.store.path().exists() {
            return Err(SecurityError::Vault(
                "数据库存在但解锁元数据缺失，请恢复对应的 vault.json 文件；不会重建或覆盖数据库",
            ));
        }
        Ok(self.status_value(&boot, header.is_some()))
    }
    async fn open_database(&self, header: &Header, key: &[u8; 32]) -> Result<bool, SecurityError> {
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
        // 数据库和派生元数据均成功发布后，才允许同步模块取得相应主密钥。
        *self.master.lock().unwrap() = Some(VaultKey {
            key: Zeroizing::new(*key),
            salt: header.salt,
        });
        self.unlocked
            .store(true, std::sync::atomic::Ordering::Release);
        let _ = self.reconcile_deletions().await;
        Ok(self
            .store
            .finish_password_change_recovery()
            .await
            .unwrap_or(true))
    }
    /// 首次设置或密码恢复均在 Rust 内派生密钥；密码、密钥不进入返回 DTO。
    pub async fn unlock_with_password(
        &self,
        password: Zeroizing<String>,
        setup: bool,
    ) -> Result<VaultStatus, SecurityError> {
        let vault = self.clone();
        // UI 或 IPC 请求取消后仍完成已开始的文件与缓存提交；进程退出由改密日志恢复。
        tokio::spawn(async move { vault.unlock_with_password_inner(password, setup).await })
            .await
            .map_err(|_| unavailable())?
    }
    async fn unlock_with_password_inner(
        &self,
        password: Zeroizing<String>,
        setup: bool,
    ) -> Result<VaultStatus, SecurityError> {
        let mut boot = self.boot.lock().await;
        if self.is_unlocked() {
            return Ok(self.status_value(&boot, true));
        }
        self.store.recover_password_change().await?;
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
        boot.cleanup_warning = cleanup_warning(self.open_database(&header, &key).await?);
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

    pub fn master_key(&self) -> Result<VaultKey, SecurityError> {
        // 文件切换可能已重新开池但尚在清理；提交协调结束前不发布旧密钥快照。
        let _boot = self.boot.try_lock().map_err(|_| SecurityError::NotReady)?;
        self.cached_master_key()
    }
    fn cached_master_key(&self) -> Result<VaultKey, SecurityError> {
        if !self.is_unlocked() {
            return Err(SecurityError::NotReady);
        }
        self.master
            .lock()
            .unwrap()
            .clone()
            .ok_or(SecurityError::NotReady)
    }

    pub async fn verify_password(
        &self,
        password: Zeroizing<String>,
    ) -> Result<VaultKey, SecurityError> {
        let _boot = self.boot.lock().await;
        if password.is_empty() || password.len() > 1024 {
            return Err(SecurityError::Vault("请输入保险库密码，且不超过1024字节"));
        }
        let header = read_header(&self.header_path())?.ok_or_else(unavailable)?;
        tokio::task::spawn_blocking(move || {
            let key = derive(&password, &header.salt)?;
            header.verify(&key)?;
            Ok(VaultKey {
                key,
                salt: header.salt,
            })
        })
        .await
        .map_err(|_| unavailable())?
    }

    /// 仅可传入已经完整认证云包的材料；既有本地库必须先用本地密码解锁。
    pub async fn adopt_master_key(&self, material: VaultKey) -> Result<(), SecurityError> {
        let vault = self.clone();
        tokio::spawn(async move { vault.adopt_master_key_inner(material).await })
            .await
            .map_err(|_| unavailable())?
    }
    async fn adopt_master_key_inner(&self, material: VaultKey) -> Result<(), SecurityError> {
        let mut boot = self.boot.lock().await;
        self.store.recover_password_change().await?;
        let existing = read_header(&self.header_path())?;
        let unlocked = self.is_unlocked();
        // 元数据单独存在也属于既有保险库；中断初始化只能凭原密码恢复。
        if !unlocked && (existing.is_some() || self.store.path().exists()) {
            return Err(SecurityError::Vault("请先解锁本地保险库后再合并官方云数据"));
        }
        let mut header = if let Some(header) = existing {
            if !header.initialized {
                return Err(SecurityError::Vault(
                    "保险库初始化尚未完成，请重新启动后重试",
                ));
            }
            header
        } else {
            if unlocked {
                return Err(unavailable());
            }
            Header {
                format: 2,
                id: uuid::Uuid::new_v4().to_string(),
                kdf: KDF.into(),
                salt: material.salt,
                nonce: [0; 12],
                verifier: Vec::new(),
                initialized: false,
            }
        };
        if unlocked {
            let current = self.cached_master_key()?;
            if current.salt != header.salt {
                return Err(unavailable());
            }
            // 不因云端认证成功而重新签封已被篡改的本地元数据。
            header.verify(&current.key)?;
        }
        header.salt = material.salt;
        header.seal(&material.key)?;
        if unlocked {
            let old = self.cached_master_key()?;
            if old.salt == material.salt && *old.key == *material.key {
                return Ok(());
            }
            let _operation = self.operations.lock().await;
            let metadata = serde_json::to_vec(&header).map_err(|_| unavailable())?;
            let changed = match self
                .store
                .change_key(&old.key, &material.key, metadata)
                .await
            {
                Ok(changed) => changed,
                Err(error) => {
                    self.reconcile_key_change_failure();
                    return Err(error.into());
                }
            };
            // 不在文件提交后重新读取元数据，避免读盘错误留下已失效的旧缓存。
            *self.master.lock().unwrap() = Some(material.clone());
            boot.cleanup_warning = cleanup_warning(changed.cleanup_pending);
        } else {
            write_header(&self.header_path(), &header, false)?;
            boot.cleanup_warning =
                cleanup_warning(self.open_database(&header, &material.key).await?);
        }
        boot.attempted = true;
        let backend = self.key_backend.clone();
        boot.warning = tokio::task::spawn_blocking(move || backend.save(material.key.as_ref()))
            .await
            .unwrap_or(Err(SecurityError::Unavailable))
            .err()
            .map(|_| "主密钥未能保存到系统钥匙串，下次需输入保险库密码。".into());
        Ok(())
    }

    fn reconcile_key_change_failure(&self) {
        let available = self.store.is_unlocked();
        self.unlocked
            .store(available, std::sync::atomic::Ordering::Release);
        if !available {
            // 恢复失败时保留磁盘恢复材料，但不向同步模块暴露无法确认对应库的密钥。
            *self.master.lock().unwrap() = None;
        }
    }

    /// 已自动解锁也必须独立验证旧密码；密码变更不复用解锁接口的快速成功分支。
    pub async fn change_password(
        &self,
        old_password: Zeroizing<String>,
        new_password: Zeroizing<String>,
    ) -> Result<PasswordChangeResult, SecurityError> {
        let vault = self.clone();
        tokio::spawn(async move {
            vault
                .change_password_inner(old_password, new_password)
                .await
        })
        .await
        .map_err(|_| unavailable())?
    }
    async fn change_password_inner(
        &self,
        old_password: Zeroizing<String>,
        new_password: Zeroizing<String>,
    ) -> Result<PasswordChangeResult, SecurityError> {
        let mut boot = self.boot.lock().await;
        if !self.is_unlocked() {
            return Err(SecurityError::Vault("请先解锁数据保险库"));
        }
        if old_password.is_empty() || old_password.len() > 1024 {
            return Err(SecurityError::Vault("请输入旧密码，且不超过1024字节"));
        }
        if new_password.chars().count() < 12 || new_password.len() > 1024 {
            return Err(SecurityError::Vault("新密码至少12个字符，且不超过1024字节"));
        }
        if old_password == new_password {
            return Err(SecurityError::Vault("新密码不能与旧密码相同"));
        }
        let header = read_header(&self.header_path())?.ok_or_else(unavailable)?;
        if !header.initialized {
            return Err(SecurityError::Vault(
                "保险库初始化尚未完成，请重新启动后重试",
            ));
        }
        let (old_key, new_material, metadata) = tokio::task::spawn_blocking(move || {
            let old_key = derive(&old_password, &header.salt)?;
            header
                .verify(&old_key)
                .map_err(|_| SecurityError::Vault("旧密码不正确或保险库校验数据损坏"))?;
            let mut next = header;
            OsRng
                .try_fill_bytes(&mut next.salt)
                .map_err(|_| unavailable())?;
            let new_key = derive(&new_password, &next.salt)?;
            next.seal(&new_key)?;
            let metadata = serde_json::to_vec(&next).map_err(|_| unavailable())?;
            Ok::<_, SecurityError>((
                old_key,
                VaultKey {
                    key: new_key,
                    salt: next.salt,
                },
                metadata,
            ))
        })
        .await
        .map_err(|_| unavailable())??;
        // 与凭据的多步写入串行，数据库内部另有生命周期门闩等待已有事务结束。
        let _operation = self.operations.lock().await;
        let changed = match self
            .store
            .change_key(&old_key, &new_material.key, metadata)
            .await
        {
            Ok(changed) => changed,
            Err(error) => {
                self.reconcile_key_change_failure();
                return Err(error.into());
            }
        };
        // 只有新的加密文件组合提交成功后，才更新唯一的 OS 密钥缓存。
        *self.master.lock().unwrap() = Some(new_material.clone());
        let backend = self.key_backend.clone();
        let remembered =
            tokio::task::spawn_blocking(move || backend.save(new_material.key.as_ref()))
                .await
                .unwrap_or(Err(SecurityError::Unavailable));
        boot.warning = remembered
            .err()
            .map(|_| "密码已修改，但新密钥未能保存到系统钥匙串；下次启动需输入新密码。".into());
        boot.cleanup_warning = cleanup_warning(changed.cleanup_pending);
        Ok(PasswordChangeResult {
            keychain_warning: boot.warning.clone(),
            cleanup_warning: boot.cleanup_warning.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vault_command_futures_are_send() {
        fn assert_send(_: impl Send) {}

        let vault = CredentialVault::new(std::sync::Arc::new(
            terminal_storage::SqliteStore::locked("unused-vault-send-check.sqlite3"),
        ));
        // 只构造并丢弃 future，不轮询，不打开数据库或访问系统钥匙串。
        assert_send(vault.status());
        assert_send(vault.unlock_with_password(Zeroizing::new(String::new()), false));
        assert_send(vault.verify_password(Zeroizing::new(String::new())));
        assert_send(vault.adopt_master_key(VaultKey {
            key: Zeroizing::new([0; 32]),
            salt: [0; 16],
        }));
        assert_send(
            vault.change_password(Zeroizing::new(String::new()), Zeroizing::new(String::new())),
        );
    }

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
                std::env::temp_dir().join(format!("termeta-vault-test-{}", uuid::Uuid::new_v4()));
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

    #[test]
    fn official_derivation_matches_local_password_and_preserves_whitespace() {
        let password = "  shared-vault-password  ";
        let (header, key) = Header::create(password).unwrap();
        let material = derive_vault_key(password, header.salt).unwrap();
        assert_eq!(*material.key, *key);
        assert_eq!(material.salt, header.salt);
        assert_ne!(
            *derive_vault_key(password.trim(), header.salt).unwrap().key,
            *key
        );
        assert!(derive_vault_key("too-short", header.salt).is_err());
        assert!(derive_vault_key(&"密".repeat(342), header.salt).is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cloud_adoption_rejects_locked_database_and_interrupted_local_metadata() {
        use std::sync::{Arc, atomic::Ordering};
        let backend = Arc::new(FakeKeychain::default());
        let path = TestPath::new();
        let original = path.vault(backend.clone());
        original
            .unlock_with_password(Zeroizing::new("local-original-password".into()), true)
            .await
            .unwrap();
        let locked = path.vault(backend.clone());
        let header_before = std::fs::read(locked.header_path()).unwrap();
        let database_before = std::fs::read(locked.store.path()).unwrap();
        let material = derive_vault_key("remote-cloud-password", [31; 16]).unwrap();
        assert!(locked.adopt_master_key(material.clone()).await.is_err());
        assert_eq!(header_before, std::fs::read(locked.header_path()).unwrap());
        assert_eq!(database_before, std::fs::read(locked.store.path()).unwrap());
        assert!(!locked.is_unlocked());
        assert!(locked.master_key().is_err());
        assert_eq!(backend.writes.load(Ordering::SeqCst), 1);

        let incomplete_path = TestPath::new();
        let incomplete = incomplete_path.vault(backend.clone());
        let (header, _) = Header::create("interrupted-local-password").unwrap();
        write_header(&incomplete.header_path(), &header, false).unwrap();
        let before = std::fs::read(incomplete.header_path()).unwrap();
        assert!(incomplete.adopt_master_key(material).await.is_err());
        assert_eq!(before, std::fs::read(incomplete.header_path()).unwrap());
        assert!(!incomplete.store.path().exists());
        assert_eq!(backend.writes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fresh_cloud_adoption_initializes_recoverable_metadata_without_keychain() {
        use std::sync::{Arc, atomic::Ordering};
        let path = TestPath::new();
        let backend = Arc::new(FakeKeychain::default());
        backend.fail_write.store(true, Ordering::SeqCst);
        let vault = path.vault(backend.clone());
        let material = derive_vault_key("remote-cloud-password", [37; 16]).unwrap();
        vault.adopt_master_key(material.clone()).await.unwrap();
        let cache = vault.master_key().unwrap();
        assert_eq!(*cache.key, *material.key);
        assert_eq!(cache.salt, material.salt);
        let header = read_header(&vault.header_path()).unwrap().unwrap();
        assert!(header.initialized);
        assert_eq!(header.salt, material.salt);
        header.verify(&material.key).unwrap();
        assert!(vault.status().await.unwrap().keychain_warning.is_some());
        let recovered = path.vault(backend);
        assert_eq!(recovered.status().await.unwrap().mode, "locked");
        recovered
            .unlock_with_password(Zeroizing::new("remote-cloud-password".into()), false)
            .await
            .unwrap();
        assert_eq!(*recovered.master_key().unwrap().key, *material.key);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cloud_adoption_preserves_credentials_and_updates_both_key_caches() {
        use std::sync::{Arc, atomic::Ordering};
        let path = TestPath::new();
        let backend = Arc::new(FakeKeychain::default());
        let vault = path.vault(backend.clone());
        vault
            .unlock_with_password(Zeroizing::new("local-original-password".into()), true)
            .await
            .unwrap();
        let credential = vault
            .save_password("保留凭据", Zeroizing::new("preserved-secret".into()))
            .await
            .unwrap();
        let before = read_header(&vault.header_path()).unwrap().unwrap();
        let material = derive_vault_key("remote-cloud-password", [43; 16]).unwrap();
        vault.adopt_master_key(material.clone()).await.unwrap();
        let cache = vault.master_key().unwrap();
        assert_eq!(*cache.key, *material.key);
        assert_eq!(cache.salt, material.salt);
        let header = read_header(&vault.header_path()).unwrap().unwrap();
        assert_eq!(header.id, before.id);
        assert_eq!(header.salt, material.salt);
        assert!(header.initialized);
        header.verify(&material.key).unwrap();
        assert_eq!(
            backend.key.lock().unwrap().as_deref(),
            Some(material.key.as_slice())
        );
        assert_eq!(
            vault.load(&credential).await.unwrap().as_str(),
            "preserved-secret"
        );
        assert_eq!(backend.writes.load(Ordering::SeqCst), 2);
        vault.adopt_master_key(material).await.unwrap();
        assert_eq!(backend.writes.load(Ordering::SeqCst), 2);

        *backend.key.lock().unwrap() = None;
        let recovered = path.vault(backend);
        assert!(
            recovered
                .unlock_with_password(Zeroizing::new("local-original-password".into()), false)
                .await
                .is_err()
        );
        recovered
            .unlock_with_password(Zeroizing::new("remote-cloud-password".into()), false)
            .await
            .unwrap();
        assert_eq!(
            recovered.load(&credential).await.unwrap().as_str(),
            "preserved-secret"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn canceling_password_change_caller_does_not_interrupt_commit_or_cache_update() {
        use std::sync::{Arc, atomic::Ordering};
        let path = TestPath::new();
        let backend = Arc::new(FakeKeychain::default());
        let vault = path.vault(backend.clone());
        vault
            .unlock_with_password(Zeroizing::new("local-original-password".into()), true)
            .await
            .unwrap();
        // 阻塞实际文件切换，让调用方取消发生在后台任务已接管且提交尚未开始时。
        let operation = vault.operations.lock().await;
        let worker = vault.clone();
        let caller = tokio::spawn(async move {
            worker
                .change_password(
                    Zeroizing::new("local-original-password".into()),
                    Zeroizing::new("changed-vault-password".into()),
                )
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                if vault.boot.try_lock().is_err() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(vault.master_key().is_err());
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        drop(operation);
        // status 等待同一个协调锁，成功返回证明后台任务完成新密钥提交及缓存写入。
        let status = tokio::time::timeout(std::time::Duration::from_secs(30), vault.status())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status.mode, "unlocked");
        let header = read_header(&vault.header_path()).unwrap().unwrap();
        let cache = vault.master_key().unwrap();
        let expected = derive_vault_key("changed-vault-password", header.salt).unwrap();
        assert_eq!(*cache.key, *expected.key);
        assert_eq!(cache.salt, expected.salt);
        header.verify(&cache.key).unwrap();
        assert_eq!(
            backend.key.lock().unwrap().as_deref(),
            Some(cache.key.as_slice())
        );
        assert_eq!(backend.writes.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn password_change_checks_old_password_and_preserves_credentials() {
        use std::sync::{Arc, atomic::Ordering};
        let path = TestPath::new();
        let backend = Arc::new(FakeKeychain::default());
        let vault = path.vault(backend.clone());
        vault
            .unlock_with_password(Zeroizing::new("original-password-123".into()), true)
            .await
            .unwrap();
        let credential = vault
            .save_password("保留的凭据", Zeroizing::new("stored-secret".into()))
            .await
            .unwrap();
        let before = std::fs::read(vault.header_path()).unwrap();
        let database_before = std::fs::read(vault.store.path()).unwrap();
        assert!(
            vault
                .change_password(
                    Zeroizing::new("wrong-password".into()),
                    Zeroizing::new("new-password-456".into())
                )
                .await
                .is_err()
        );
        assert!(
            vault
                .change_password(
                    Zeroizing::new("original-password-123".into()),
                    Zeroizing::new("short".into())
                )
                .await
                .is_err()
        );
        assert!(
            vault
                .change_password(
                    Zeroizing::new("original-password-123".into()),
                    Zeroizing::new("original-password-123".into())
                )
                .await
                .is_err()
        );
        assert_eq!(before, std::fs::read(vault.header_path()).unwrap());
        assert_eq!(database_before, std::fs::read(vault.store.path()).unwrap());
        assert_eq!(backend.writes.load(Ordering::SeqCst), 1);
        let original = read_header(&vault.header_path()).unwrap().unwrap();
        let changed = vault
            .change_password(
                Zeroizing::new("original-password-123".into()),
                Zeroizing::new("  新的保险库密码-password-456  ".into()),
            )
            .await
            .unwrap();
        assert!(changed.keychain_warning.is_none());
        assert!(changed.cleanup_warning.is_none());
        let next = read_header(&vault.header_path()).unwrap().unwrap();
        assert_eq!(next.id, original.id);
        assert_ne!(next.salt, original.salt);
        assert_ne!(next.nonce, original.nonce);
        assert_eq!(
            vault.load(&credential).await.unwrap().as_str(),
            "stored-secret"
        );
        assert_eq!(backend.writes.load(Ordering::SeqCst), 2);
        let automatic = path.vault(backend.clone());
        assert_eq!(automatic.status().await.unwrap().mode, "unlocked");
        *backend.key.lock().unwrap() = None;
        let recovered = path.vault(backend);
        assert_eq!(recovered.status().await.unwrap().mode, "locked");
        assert!(
            recovered
                .unlock_with_password(Zeroizing::new("original-password-123".into()), false)
                .await
                .is_err()
        );
        assert!(
            recovered
                .unlock_with_password(Zeroizing::new("新的保险库密码-password-456".into()), false)
                .await
                .is_err()
        );
        recovered
            .unlock_with_password(
                Zeroizing::new("  新的保险库密码-password-456  ".into()),
                false,
            )
            .await
            .unwrap();
        assert_eq!(
            recovered.load(&credential).await.unwrap().as_str(),
            "stored-secret"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn changed_password_survives_keychain_write_failure() {
        use std::sync::{Arc, atomic::Ordering};
        let path = TestPath::new();
        let backend = Arc::new(FakeKeychain::default());
        let vault = path.vault(backend.clone());
        vault
            .unlock_with_password(Zeroizing::new("original-password-123".into()), true)
            .await
            .unwrap();
        let old_cached_key = backend.key.lock().unwrap().clone();
        backend.fail_write.store(true, Ordering::SeqCst);
        let changed = vault
            .change_password(
                Zeroizing::new("original-password-123".into()),
                Zeroizing::new("new-password-456".into()),
            )
            .await
            .unwrap();
        assert!(changed.keychain_warning.is_some());
        assert_eq!(old_cached_key, *backend.key.lock().unwrap());
        let current = vault.master_key().unwrap();
        let header = read_header(&vault.header_path()).unwrap().unwrap();
        assert_eq!(current.salt, header.salt);
        assert_eq!(
            *current.key,
            *derive_vault_key("new-password-456", header.salt)
                .unwrap()
                .key
        );
        header.verify(&current.key).unwrap();
        let recovered = path.vault(backend);
        assert_eq!(recovered.status().await.unwrap().mode, "locked");
        assert!(
            recovered
                .unlock_with_password(Zeroizing::new("original-password-123".into()), false)
                .await
                .is_err()
        );
        assert_eq!(
            recovered
                .unlock_with_password(Zeroizing::new("new-password-456".into()), false)
                .await
                .unwrap()
                .mode,
            "unlocked"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn concurrent_changes_cannot_overwrite_a_newer_password() {
        let path = TestPath::new();
        let backend = std::sync::Arc::new(FakeKeychain::default());
        let vault = path.vault(backend.clone());
        vault
            .unlock_with_password(Zeroizing::new("original-password-123".into()), true)
            .await
            .unwrap();
        let (first, second) = tokio::join!(
            vault.change_password(
                Zeroizing::new("original-password-123".into()),
                Zeroizing::new("first-new-password".into())
            ),
            vault.change_password(
                Zeroizing::new("original-password-123".into()),
                Zeroizing::new("second-new-password".into())
            ),
        );
        assert_ne!(first.is_ok(), second.is_ok());
        let winner = if first.is_ok() {
            "first-new-password"
        } else {
            "second-new-password"
        };
        *backend.key.lock().unwrap() = None;
        let recovered = path.vault(backend);
        recovered
            .unlock_with_password(Zeroizing::new(winner.into()), false)
            .await
            .unwrap();
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

/// 官方包和本机保险库使用完全相同的派生参数和主密钥。
pub fn derive_vault_key(password: &str, salt: [u8; 16]) -> Result<VaultKey, SecurityError> {
    if password.chars().count() < 12 || password.len() > 1024 {
        return Err(SecurityError::Vault(
            "保险库密码至少12个字符，且不超过1024字节",
        ));
    }
    Ok(VaultKey {
        key: derive(password, &salt)?,
        salt,
    })
}

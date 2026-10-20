//! 保险库状态、解锁与主密钥访问；认证元数据和密钥变更分别封装。
use crate::{CredentialVault, SecurityError};
use serde::Serialize;
use std::path::PathBuf;
use zeroize::Zeroizing;

mod metadata;
mod rotation;
#[cfg(test)]
mod tests;

use metadata::{Header, derive, read_header, write_header};

/// 仅在 Rust 内存持有；不得进入 WebView、云配置或日志。
#[derive(Clone)]
pub struct VaultKey {
    pub key: Zeroizing<[u8; 32]>,
    pub salt: [u8; 16],
}

const SERVICE: &str = "io.termeta";
const ACCOUNT: &str = "master-key-v2";
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
fn unavailable() -> SecurityError {
    SecurityError::Vault("保险库元数据不可用或已损坏，请保留文件后重试")
}
fn cleanup_warning(pending: bool) -> Option<String> {
    pending.then(|| {
        "保险库恢复文件尚未完全清理；退出后重新启动将重试清理。旧副本仍受原密码保护。".into()
    })
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

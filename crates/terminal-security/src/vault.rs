use async_trait::async_trait;
use std::{path::Path, sync::Arc};
use terminal_core::{CredentialStore, SessionError};
use terminal_storage::SqliteStore;
use thiserror::Error;
use tokio::sync::Mutex;
use zeroize::Zeroizing;

const SERVICE: &str = "com.h3110w0r1d.termvexa";

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("系统凭据库不可用、已锁定或访问被拒绝；可以选择仅本次使用")]
    Unavailable,
    #[error("凭据元数据更新失败，保留待处理状态")]
    Metadata(#[from] terminal_storage::StorageError),
    #[error("凭据不存在或尚未就绪")]
    NotReady,
    #[error("私钥文件不可读、格式不受支持、权限过宽或内容已变更，请重新选择授权")]
    KeyFile,
    #[error("调用窗口无权使用该资源")]
    Forbidden,
}

pub struct CredentialVault {
    store: Arc<SqliteStore>,
    // 系统 API 可阻塞或显示授权框，仅在阻塞工作池执行；串行化避免同项竞态。
    operations: Mutex<()>,
}
impl CredentialVault {
    pub fn new(store: Arc<SqliteStore>) -> Self {
        Self {
            store,
            operations: Mutex::new(()),
        }
    }
    pub async fn save(
        &self,
        kind: &str,
        label: &str,
        secret: Zeroizing<String>,
    ) -> Result<String, SecurityError> {
        if secret.is_empty() || secret.len() > 65536 {
            return Err(SecurityError::NotReady);
        }
        let _guard = self.operations.lock().await;
        let id = self.store.begin_credential(kind, label).await?;
        let account = id.clone();
        tokio::task::spawn_blocking(move || {
            keyring::Entry::new(SERVICE, &account)
                .map_err(|_| SecurityError::Unavailable)?
                .set_password(&secret)
                .map_err(|_| SecurityError::Unavailable)
        })
        .await
        .map_err(|_| SecurityError::Unavailable)??;
        // 若此步失败，pending 仍留在数据库，不把半完成写入提供给连接使用。
        self.store.credential_ready(&id).await?;
        Ok(id)
    }
    pub async fn load(&self, id: &str) -> Result<Zeroizing<String>, SecurityError> {
        let _guard = self.operations.lock().await;
        if self.store.get_credential(id).await?.status != "ready" {
            return Err(SecurityError::NotReady);
        }
        let account = id.to_owned();
        tokio::task::spawn_blocking(move || {
            let entry =
                keyring::Entry::new(SERVICE, &account).map_err(|_| SecurityError::Unavailable)?;
            entry
                .get_password()
                .map(Zeroizing::new)
                .map_err(|_| SecurityError::Unavailable)
        })
        .await
        .map_err(|_| SecurityError::Unavailable)?
    }
    pub async fn delete(&self, id: &str) -> Result<(), SecurityError> {
        let _guard = self.operations.lock().await;
        self.store.begin_delete_credential(id).await?;
        self.delete_item(id).await?;
        self.store.finish_delete_credential(id).await?;
        Ok(())
    }
    async fn delete_item(&self, id: &str) -> Result<(), SecurityError> {
        let account = id.to_owned();
        tokio::task::spawn_blocking(move || {
            let entry =
                keyring::Entry::new(SERVICE, &account).map_err(|_| SecurityError::Unavailable)?;
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(_) => Err(SecurityError::Unavailable),
            }
        })
        .await
        .map_err(|_| SecurityError::Unavailable)?
    }
    /// 只协调当前数据库已登记的删除任务，不扫描系统凭据库。
    pub async fn reconcile_deletions(&self) -> Result<(), SecurityError> {
        let _guard = self.operations.lock().await;
        for item in self.store.list_credentials().await? {
            if item.status == "deleting" {
                self.delete_item(&item.id).await?;
                self.store.finish_delete_credential(&item.id).await?;
            }
        }
        Ok(())
    }
    /// 用户明确确认后，才将先前中断且 OS 项存在的 pending 项恢复为 ready。
    pub async fn recover_pending(&self, id: &str) -> Result<(), SecurityError> {
        let _guard = self.operations.lock().await;
        if self.store.get_credential(id).await?.status != "pending" {
            return Err(SecurityError::NotReady);
        }
        let account = id.to_owned();
        let _secret = tokio::task::spawn_blocking(move || {
            keyring::Entry::new(SERVICE, &account)
                .map_err(|_| SecurityError::Unavailable)?
                .get_password()
                .map(Zeroizing::new)
                .map_err(|_| SecurityError::Unavailable)
        })
        .await
        .map_err(|_| SecurityError::Unavailable)??;
        self.store.credential_ready(id).await?;
        Ok(())
    }
}
fn session_error(_: SecurityError) -> SessionError {
    SessionError::new(
        "CREDENTIAL_UNAVAILABLE",
        "authentication",
        "凭据或已授权私钥不可用，请重新选择或使用本次认证",
    )
}
#[async_trait]
impl CredentialStore for CredentialVault {
    async fn secret(&self, owner: &str, id: &str) -> terminal_core::Result<Zeroizing<String>> {
        if owner != "main" {
            return Err(session_error(SecurityError::Forbidden));
        }
        self.load(id).await.map_err(session_error)
    }
    async fn key_file(
        &self,
        owner: &str,
        key_file_ref: &str,
    ) -> terminal_core::Result<Zeroizing<String>> {
        if owner != "main" {
            return Err(session_error(SecurityError::Forbidden));
        }
        let file = self
            .store
            .get_key_file(key_file_ref)
            .await
            .map_err(|_| session_error(SecurityError::KeyFile))?;
        tokio::task::spawn_blocking(move || {
            crate::private_key::read_key(Path::new(&file.path), file.content_fingerprint())
        })
        .await
        .map_err(|_| session_error(SecurityError::KeyFile))?
        .map_err(session_error)
    }
}

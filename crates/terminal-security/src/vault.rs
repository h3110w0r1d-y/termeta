use async_trait::async_trait;
use serde::Deserialize;
use std::{path::Path, sync::Arc};
use terminal_core::{AuthStep, CredentialStore, PrivateKeyMaterial, SessionError};
use terminal_storage::SqliteStore;
use terminal_storage::{HostDraft, HostProfile};
use thiserror::Error;
use tokio::sync::Mutex;
use zeroize::Zeroizing;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("{0}")]
    Vault(&'static str),
    #[error("系统凭据库不可用、已锁定或访问被拒绝")]
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

#[derive(Clone)]
pub struct CredentialVault {
    pub(crate) store: Arc<SqliteStore>,
    pub(crate) unlocked: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) key_backend: Arc<dyn crate::master_key::MasterKeyBackend>,
    pub(crate) boot: Arc<Mutex<crate::master_key::BootState>>,
    pub(crate) master: Arc<std::sync::Mutex<Option<crate::master_key::VaultKey>>>,
    // 凭据写入与引用替换串行协调；钥匙串仅在主密钥解锁与改密流程使用。
    pub(crate) operations: Arc<Mutex<()>>,
}
impl CredentialVault {
    pub fn new(store: Arc<SqliteStore>) -> Self {
        Self {
            store,
            unlocked: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            key_backend: Arc::new(crate::master_key::OsMasterKey),
            boot: Arc::new(Mutex::new(Default::default())),
            master: Arc::new(std::sync::Mutex::new(None)),
            operations: Arc::new(Mutex::new(())),
        }
    }
    pub async fn save_password(
        &self,
        label: &str,
        secret: Zeroizing<String>,
    ) -> Result<String, SecurityError> {
        if secret.is_empty() || secret.len() > 65536 {
            return Err(SecurityError::NotReady);
        }
        let _guard = self.operations.lock().await;
        Ok(self
            .store
            .save_public_credential("password", label, &secret)
            .await?)
    }
    pub async fn save_private_key(
        &self,
        label: &str,
        key_file_ref: &str,
        passphrase: Option<Zeroizing<String>>,
    ) -> Result<String, SecurityError> {
        if passphrase.as_ref().is_some_and(|value| value.len() > 65536) {
            return Err(SecurityError::NotReady);
        }
        let _guard = self.operations.lock().await;
        let file = self.store.get_key_file(key_file_ref).await?;
        let text = tokio::task::spawn_blocking(move || {
            crate::private_key::read_key(Path::new(&file.path), file.content_fingerprint())
        })
        .await
        .map_err(|_| SecurityError::KeyFile)??;
        // 整体存成单个加密数据库凭据项，口令不能成为可独立引用的凭据。
        // 按 JSON 最坏转义长度预留空间，避免扩容残留未擦除的私钥副本。
        let capacity = 6 * (text.len() + passphrase.as_ref().map_or(0, |value| value.len())) + 128;
        let mut bytes = Zeroizing::new(Vec::with_capacity(capacity));
        serde_json::to_writer(
            &mut *bytes,
            &KeyPayloadRef {
                private_key: text.as_str(),
                passphrase: passphrase.as_ref().map(|value| value.as_str()),
            },
        )
        .map_err(|_| SecurityError::KeyFile)?;
        let payload = Zeroizing::new(
            String::from_utf8(std::mem::take(&mut *bytes)).map_err(|_| SecurityError::KeyFile)?,
        );
        Ok(self
            .store
            .save_public_credential("privateKey", label, &payload)
            .await?)
    }
    pub async fn save_host(
        &self,
        host: HostDraft,
        password: Option<Zeroizing<String>>,
    ) -> Result<HostProfile, SecurityError> {
        let _guard = self.operations.lock().await;
        // 非 SSH 配置不接受任何密码字段，避免凭据被悄悄忽略或写入。
        if host.kind != "ssh" && password.is_some() {
            return Err(
                terminal_storage::StorageError::Validation("非 SSH 主机不支持密码凭据").into(),
            );
        }
        let Some(password) = password else {
            let saved = self.store.save_host(host).await?;
            // 主机事务已提交；数据库清理失败保留 deleting，不能把成功保存误报为失败。
            let _ = self.reconcile_locked().await;
            return Ok(saved);
        };
        if password.len() > 65536 {
            return Err(SecurityError::NotReady);
        }
        let direct = matches!(
            host.auth_plan.first(),
            Some(AuthStep::Password {
                credential_id: None
            })
        );
        if !direct || password.is_empty() {
            let saved = self.store.save_host_with_password_ref(host, None).await?;
            // 主机事务已提交；数据库清理失败保留 deleting，不能把成功保存误报为失败。
            let _ = self.reconcile_locked().await;
            return Ok(saved);
        }
        let profile_id = host
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let id = self
            .store
            .begin_host_password(&profile_id, &host.name)
            .await?;
        if let Err(error) = self.store.set_credential_secret(&id, &password).await {
            // 未绑定的新项无法供任何会话使用，清理失败仍保留 deleting 状态。
            if self.store.begin_delete_credential(&id).await.is_ok() {
                let _ = self.reconcile_locked().await;
            }
            return Err(error.into());
        }
        // 先写新项，再事务替换引用；失败时不改动旧项，孤立新项由删除协调回收。
        let result = self
            .store
            .save_host_with_password_ref(host, Some(&id))
            .await;
        if result.is_err() {
            if self.store.begin_delete_credential(&id).await.is_ok() {
                let _ = self.reconcile_locked().await;
            }
        }
        let saved = result?;
        // 清理旧项不改变本次事务成功语义，后续启动继续协调删除。
        let _ = self.reconcile_locked().await;
        Ok(saved)
    }
    pub async fn load(&self, id: &str) -> Result<Zeroizing<String>, SecurityError> {
        let _guard = self.operations.lock().await;
        let item = self.store.get_credential(id).await?;
        if item.status != "ready" || item.kind != "password" {
            return Err(SecurityError::NotReady);
        }
        self.store
            .credential_secret(id)
            .await
            .map_err(SecurityError::from)
    }

    pub async fn delete(&self, id: &str) -> Result<(), SecurityError> {
        let _guard = self.operations.lock().await;
        if self
            .store
            .get_credential(id)
            .await?
            .owner_profile_id
            .is_some()
        {
            return Err(SecurityError::Forbidden);
        }
        self.store.begin_delete_credential(id).await?;
        self.delete_item(id).await?;
        self.store.finish_delete_credential(id).await?;
        Ok(())
    }
    async fn delete_item(&self, id: &str) -> Result<(), SecurityError> {
        self.store.erase_credential_secret(id).await?;
        Ok(())
    }
    /// 只协调当前加密数据库已登记的删除任务。
    pub async fn reconcile_deletions(&self) -> Result<(), SecurityError> {
        let _guard = self.operations.lock().await;
        self.reconcile_locked().await
    }
    async fn reconcile_locked(&self) -> Result<(), SecurityError> {
        self.store.mark_orphaned_credentials_deleting().await?;
        let mut first_error = None;
        for item in self.store.list_all_credentials().await? {
            if item.status != "deleting" {
                continue;
            }
            // 单项失败不阻止其他独立条目清理，失败项保持 deleting 供下次重试。
            let result = match self.delete_item(&item.id).await {
                Ok(()) => self
                    .store
                    .finish_delete_credential(&item.id)
                    .await
                    .map_err(SecurityError::from),
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
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
    async fn private_key(
        &self,
        owner: &str,
        id: &str,
    ) -> terminal_core::Result<PrivateKeyMaterial> {
        if owner != "main" {
            return Err(session_error(SecurityError::Forbidden));
        }
        let _guard = self.operations.lock().await;
        let item = self
            .store
            .get_credential(id)
            .await
            .map_err(|_| session_error(SecurityError::NotReady))?;
        if item.kind != "privateKey" || item.status != "ready" || item.owner_profile_id.is_some() {
            return Err(session_error(SecurityError::NotReady));
        }
        let text = self
            .store
            .credential_secret(id)
            .await
            .map_err(|_| session_error(SecurityError::NotReady))?;
        let material: KeyPayload =
            serde_json::from_str(&text).map_err(|_| session_error(SecurityError::KeyFile))?;
        Ok(PrivateKeyMaterial {
            private_key: material.private_key,
            passphrase: material.passphrase,
        })
    }
}
#[derive(serde::Serialize)]
struct KeyPayloadRef<'a> {
    private_key: &'a str,
    passphrase: Option<&'a str>,
}
#[derive(Deserialize)]
struct KeyPayload {
    #[serde(deserialize_with = "secret_text")]
    private_key: Zeroizing<String>,
    #[serde(deserialize_with = "optional_secret_text")]
    passphrase: Option<Zeroizing<String>>,
}
fn secret_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Zeroizing<String>, D::Error> {
    String::deserialize(deserializer).map(Zeroizing::new)
}
fn optional_secret_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Zeroizing<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(|value| value.map(Zeroizing::new))
}

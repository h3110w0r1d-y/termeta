//! 协调密码变更，保留文件切换、系统缓存和恢复的提交顺序。
use super::metadata::{derive, read_header};
use super::{PasswordChangeResult, cleanup_warning, unavailable};
use crate::{CredentialVault, SecurityError};
use rand_core::{OsRng, RngCore};
use zeroize::Zeroizing;

impl CredentialVault {
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
        let (old_key, new_key, metadata) = tokio::task::spawn_blocking(move || {
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
            Ok::<_, SecurityError>((old_key, new_key, metadata))
        })
        .await
        .map_err(|_| unavailable())??;
        // 与凭据的多步写入串行，数据库内部另有生命周期门闩等待已有事务结束。
        let _operation = self.operations.lock().await;
        let changed = match self.store.change_key(&old_key, &new_key, metadata).await {
            Ok(changed) => changed,
            Err(error) => {
                self.unlocked.store(
                    self.store.is_unlocked(),
                    std::sync::atomic::Ordering::Release,
                );
                return Err(error.into());
            }
        };
        // 只有新的加密文件组合提交成功后，才更新唯一的 OS 密钥缓存。
        let backend = self.key_backend.clone();
        let remembered = tokio::task::spawn_blocking(move || backend.save(new_key.as_ref()))
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

//! 协调已认证主密钥替换与密码变更，保留文件切换、缓存和恢复的提交顺序。
use super::metadata::{Header, KDF, derive, read_header, write_header};
use super::{PasswordChangeResult, VaultKey, cleanup_warning, unavailable};
use crate::{CredentialVault, SecurityError};
use rand_core::{OsRng, RngCore};
use zeroize::Zeroizing;

impl CredentialVault {
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

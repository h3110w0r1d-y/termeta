use crate::*;
use async_trait::async_trait;
use terminal_core::{RemoteSystemStore, SshConfig};

#[async_trait]
impl RemoteSystemStore for SqliteStore {
    async fn claim(
        &self,
        id: &str,
        token: &str,
        config: &SshConfig,
    ) -> terminal_core::Result<bool> {
        let result = async {
            // 开关与一次性标记在同一写事务检查，关闭功能不消耗尝试机会。
            let mut tx = self.pool().await?.begin_with("BEGIN IMMEDIATE").await?;
            let setting: Option<String> = sqlx::query_scalar("SELECT value_json FROM settings WHERE key='app'")
                .fetch_optional(&mut *tx).await?;
            if let Some(setting) = setting {
                let value: serde_json::Value = serde_json::from_str(&setting)?;
                if value["detectSshSystem"] == false { return Ok(false); }
            }
            let changed = sqlx::query("UPDATE connection_profiles SET config_json=json_set(config_json,'$.systemDetectionAttempted',json('true'),'$.systemDetectionToken',?),revision=revision+1 WHERE id=? AND kind='ssh' AND coalesce(json_extract(config_json,'$.systemDetectionAttempted'),0)=0 AND coalesce(json_extract(config_json,'$.icon'),'ssh')='ssh' AND EXISTS(SELECT 1 FROM ssh_profiles WHERE profile_id=? AND address=? AND port=? AND username=?)")
                .bind(token).bind(id).bind(id).bind(&config.address).bind(i64::from(config.port)).bind(&config.username)
                .execute(&mut *tx).await?;
            tx.commit().await?;
            Ok::<_, StorageError>(changed.rows_affected() == 1)
        }.await;
        result.map_err(|_| {
            terminal_core::SessionError::new(
                "STORAGE_UNAVAILABLE",
                "systemDetection",
                "无法保存系统检测状态",
            )
        })
    }
    async fn complete(
        &self,
        id: &str,
        token: &str,
        config: &SshConfig,
        icon: Option<&str>,
    ) -> terminal_core::Result<()> {
        let icon = icon.unwrap_or("ssh");
        if !terminal_core::HOST_SYSTEM_ICONS.contains(&icon) {
            return Err(terminal_core::SessionError::new(
                "INVALID_CONFIG",
                "systemDetection",
                "系统图标无效",
            ));
        }
        let result = async {
            sqlx::query("UPDATE connection_profiles SET config_json=json_set(config_json,'$.icon',?),revision=revision+1,updated_at=? WHERE id=? AND kind='ssh' AND json_extract(config_json,'$.systemDetectionToken')=? AND coalesce(json_extract(config_json,'$.icon'),'ssh')='ssh' AND EXISTS(SELECT 1 FROM ssh_profiles WHERE profile_id=? AND address=? AND port=? AND username=?)")
                .bind(icon).bind(now()).bind(id).bind(token).bind(id).bind(&config.address).bind(i64::from(config.port)).bind(&config.username)
                .execute(&*self.pool().await?).await?;
            Ok::<_, StorageError>(())
        }.await;
        result.map_err(|_| {
            terminal_core::SessionError::new(
                "STORAGE_UNAVAILABLE",
                "systemDetection",
                "无法保存系统图标",
            )
        })
    }
}

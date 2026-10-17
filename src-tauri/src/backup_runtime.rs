//! 协调云端配置、短期恢复凭证与资源预算，不向前端传递数据库正文。

use crate::{
    backup::{self, BackupProviderKind, BackupTarget},
    state::{AppState, storage_error},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use terminal_core::{Result, SessionError};
use terminal_security::backup::BackupMetadata;
use terminal_storage::{BackupImportSummary, BackupSettingWrite};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use zeroize::{Zeroize, Zeroizing};

const CONFIG_KEY: &str = "backup";
const PREVIEW_LIFETIME: Duration = Duration::from_secs(300);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupConfigDraft {
    provider: BackupProviderKind,
    endpoint: String,
    username: String,
    secret: Option<String>,
    backup_password: Option<String>,
    device_name: String,
}

impl Drop for BackupConfigDraft {
    fn drop(&mut self) {
        if let Some(secret) = &mut self.secret {
            secret.zeroize();
        }
        if let Some(password) = &mut self.backup_password {
            password.zeroize();
        }
    }
}

// 备份密码属于应用配置，提供方只能接收不含此密码的 target。
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupConfiguration {
    pub target: BackupTarget,
    backup_password: String,
}

impl Drop for BackupConfiguration {
    fn drop(&mut self) {
        self.backup_password.zeroize();
    }
}

fn validate_password(password: &str) -> Result<()> {
    if password.chars().count() < 12 || password.len() > 1024 {
        return Err(failure(
            "请配置备份密码：至少12个字符，UTF-8编码不超过1024字节",
        ));
    }
    Ok(())
}

impl BackupConfiguration {
    pub fn password(&self) -> Result<Zeroizing<String>> {
        validate_password(&self.backup_password)?;
        Ok(Zeroizing::new(self.backup_password.clone()))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupConfigView {
    provider: BackupProviderKind,
    endpoint: String,
    username: String,
    device_name: String,
    has_secret: bool,
    has_backup_password: bool,
}

impl From<&BackupConfiguration> for BackupConfigView {
    fn from(config: &BackupConfiguration) -> Self {
        let target = &config.target;
        Self {
            provider: target.provider,
            endpoint: target.endpoint.clone().unwrap_or_default(),
            username: target.username.clone().unwrap_or_default(),
            device_name: target.device_name.clone(),
            has_secret: !target.secret.is_empty(),
            has_backup_password: !config.backup_password.is_empty(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupListEntry {
    pub id: String,
    pub size: Option<u64>,
    pub metadata: Option<BackupMetadata>,
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestorePreview {
    pub token: String,
    pub metadata: BackupMetadata,
}

pub struct PreparedRestore {
    pub owner: String,
    pub initializing: bool,
    pub config: BackupConfiguration,
    pub database: Zeroizing<Vec<u8>>,
    created: Instant,
}

struct CompletedRestore {
    owner: String,
    created: Instant,
    summary: BackupImportSummary,
}

pub struct BackupRuntime {
    operation: Arc<Semaphore>,
    pub initialization: tokio::sync::Mutex<()>,
    owners: Mutex<HashMap<String, u64>>,
    prepared: Mutex<HashMap<String, PreparedRestore>>,
    completed: Mutex<HashMap<String, CompletedRestore>>,
}

impl Default for BackupRuntime {
    fn default() -> Self {
        Self {
            operation: Arc::new(Semaphore::new(1)),
            initialization: tokio::sync::Mutex::new(()),
            owners: Mutex::new(HashMap::new()),
            prepared: Mutex::new(HashMap::new()),
            completed: Mutex::new(HashMap::new()),
        }
    }
}

impl BackupRuntime {
    pub fn begin(&self) -> Result<OwnedSemaphorePermit> {
        self.operation
            .clone()
            .try_acquire_owned()
            .map_err(|_| failure("另一项备份或同步正在进行，请等待完成后重试"))
    }

    pub fn cleanup(&self) {
        self.prepared
            .lock()
            .unwrap()
            .retain(|_, value| value.created.elapsed() < PREVIEW_LIFETIME);
        self.completed
            .lock()
            .unwrap()
            .retain(|_, value| value.created.elapsed() < PREVIEW_LIFETIME);
    }

    pub fn owner_generation(&self, owner: &str) -> u64 {
        *self.owners.lock().unwrap().entry(owner.into()).or_default()
    }

    pub fn clear_owner(&self, owner: &str) {
        // 代次与插入共用短时锁，拒绝窗口销毁或配置变更后迟到的解密结果。
        let mut owners = self.owners.lock().unwrap();
        let generation = owners.entry(owner.into()).or_default();
        *generation = generation.wrapping_add(1);
        self.prepared
            .lock()
            .unwrap()
            .retain(|_, value| value.owner != owner);
    }

    pub fn prepare(
        &self,
        owner: String,
        generation: u64,
        initializing: bool,
        config: BackupConfiguration,
        opened: terminal_security::backup::OpenedBackup,
    ) -> Result<BackupRestorePreview> {
        self.cleanup();
        let owners = self.owners.lock().unwrap();
        if owners.get(&owner).copied().unwrap_or_default() != generation {
            return Err(failure("恢复窗口或配置已变化，请重新选择备份"));
        }
        // 每个窗口最多持有一份明文快照，避免列表操作累积大数据库。
        self.prepared
            .lock()
            .unwrap()
            .retain(|_, value| value.owner != owner);
        let token = uuid::Uuid::new_v4().to_string();
        let metadata = opened.metadata;
        self.prepared.lock().unwrap().insert(
            token.clone(),
            PreparedRestore {
                owner,
                initializing,
                config,
                database: opened.database,
                created: Instant::now(),
            },
        );
        Ok(BackupRestorePreview { token, metadata })
    }

    pub fn take(&self, owner: &str, token: &str, initializing: bool) -> Result<PreparedRestore> {
        self.cleanup();
        let mut prepared = self.prepared.lock().unwrap();
        let entry = prepared
            .get(token)
            .ok_or_else(|| failure("恢复预览已失效，请重新选择云端备份"))?;
        if entry.owner != owner || entry.initializing != initializing {
            return Err(failure("此窗口无权提交该恢复预览"));
        }
        prepared
            .remove(token)
            .ok_or_else(|| failure("恢复预览已失效"))
    }

    pub fn completed(&self, owner: &str, token: &str) -> Option<BackupImportSummary> {
        self.cleanup();
        self.completed
            .lock()
            .unwrap()
            .get(token)
            .filter(|entry| entry.owner == owner)
            .map(|entry| entry.summary.clone())
    }

    pub fn finish(&self, owner: String, token: String, summary: BackupImportSummary) {
        self.cleanup();
        let mut completed = self.completed.lock().unwrap();
        // 已完成凭证只保留少量摘要，用于应答丢失后的幂等重试。
        if completed.len() >= 16 {
            completed.clear();
        }
        completed.insert(
            token,
            CompletedRestore {
                owner,
                created: Instant::now(),
                summary,
            },
        );
    }
}

pub fn failure(message: &str) -> SessionError {
    SessionError::new("BACKUP_FAILED", "backup", message)
}

pub async fn saved_config(state: &AppState) -> Result<Option<BackupConfiguration>> {
    let Some(setting) = state
        .store
        .get_setting(CONFIG_KEY)
        .await
        .map_err(storage_error)?
    else {
        return Ok(None);
    };
    decode_setting(setting.value_version, setting.value).map(Some)
}

fn decode_setting(version: i64, value: serde_json::Value) -> Result<BackupConfiguration> {
    let config = match version {
        // 旧配置未保存备份密码；保留原云凭据，允许界面补填后按版本2保存。
        1 => BackupConfiguration {
            target: serde_json::from_value(value)
                .map_err(|_| failure("云备份配置无效，请重新配置"))?,
            backup_password: String::new(),
        },
        2 => serde_json::from_value(value).map_err(|_| failure("云备份配置无效，请重新配置"))?,
        _ => return Err(failure("云备份配置版本不受支持，已停止读取")),
    };
    backup::validate_target(&config.target).map_err(|error| failure(&error))?;
    if !config.backup_password.is_empty() {
        validate_password(&config.backup_password)?;
    }
    Ok(config)
}

pub async fn resolve_config(
    state: &AppState,
    draft: Option<BackupConfigDraft>,
    initializing: bool,
) -> Result<BackupConfiguration> {
    let saved = if initializing {
        None
    } else {
        saved_config(state).await?
    };
    merge_config(saved, draft)
}

fn merge_config(
    saved: Option<BackupConfiguration>,
    draft: Option<BackupConfigDraft>,
) -> Result<BackupConfiguration> {
    let Some(draft) = draft else {
        return saved.ok_or_else(|| failure("请先配置云备份目标"));
    };
    let endpoint = (!draft.endpoint.is_empty()).then(|| draft.endpoint.clone());
    let username = (!draft.username.is_empty()).then(|| draft.username.clone());
    let matching = saved.as_ref().filter(|saved| {
        saved.target.provider == draft.provider
            && saved.target.endpoint == endpoint
            && saved.target.username == username
    });
    // 仅同一认证目标允许省略秘密；更换地址不得把旧凭据发送给新服务。
    let secret = draft
        .secret
        .as_ref()
        .cloned()
        .or_else(|| matching.map(|saved| saved.target.secret.clone()))
        .ok_or_else(|| failure("请输入新目标的认证凭据"))?;
    let backup_password = draft
        .backup_password
        .as_ref()
        .cloned()
        .or_else(|| matching.map(|saved| saved.backup_password.clone()))
        .unwrap_or_default();
    let config = BackupConfiguration {
        target: BackupTarget {
            provider: draft.provider,
            endpoint,
            username,
            secret,
            device_name: draft.device_name.clone(),
        },
        backup_password,
    };
    backup::validate_target(&config.target).map_err(|error| failure(&error))?;
    if !config.backup_password.is_empty() {
        validate_password(&config.backup_password)?;
    }
    Ok(config)
}

pub fn config_setting(config: &BackupConfiguration) -> Result<BackupSettingWrite> {
    validate_password(&config.backup_password)?;
    Ok(BackupSettingWrite {
        key: CONFIG_KEY.into(),
        version: 2,
        value: serde_json::to_value(config).map_err(|_| failure("无法保存云备份配置"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_security::backup::OpenedBackup;
    use terminal_storage::BackupCounts;

    fn legacy_value() -> serde_json::Value {
        serde_json::json!({
            "provider": "gist", "endpoint": null, "username": null,
            "secret": "测试云端Token占位", "deviceName": "测试设备"
        })
    }

    fn draft(password: Option<&str>) -> BackupConfigDraft {
        BackupConfigDraft {
            provider: BackupProviderKind::Gist,
            endpoint: String::new(),
            username: String::new(),
            secret: None,
            backup_password: password.map(str::to_owned),
            device_name: "另一台设备".into(),
        }
    }

    #[test]
    fn legacy_config_keeps_token_and_upgrades_with_saved_password_without_echoing_secrets() {
        let old = decode_setting(1, legacy_value()).unwrap();
        assert!(old.password().is_err());
        assert!(!BackupConfigView::from(&old).has_backup_password);
        let config =
            merge_config(Some(old), Some(draft(Some("  仅供测试的独立备份密码  ")))).unwrap();
        let setting = config_setting(&config).unwrap();
        assert_eq!(setting.version, 2);
        let reopened = decode_setting(setting.version, setting.value).unwrap();
        assert_eq!(
            reopened.password().unwrap().as_str(),
            "  仅供测试的独立备份密码  "
        );
        assert_eq!(reopened.target.secret, "测试云端Token占位");
        let view = serde_json::to_value(BackupConfigView::from(&reopened)).unwrap();
        assert_eq!(view["hasBackupPassword"], true);
        assert_eq!(view["hasSecret"], true);
        assert!(view.get("backupPassword").is_none());
        assert!(view.get("secret").is_none());
    }

    #[test]
    fn same_target_omits_secrets_and_changed_target_requires_password_configuration() {
        let saved = merge_config(
            Some(decode_setting(1, legacy_value()).unwrap()),
            Some(draft(Some("仅供测试的独立备份密码内容"))),
        )
        .unwrap();
        let saved = merge_config(Some(saved), Some(draft(None))).unwrap();
        assert_eq!(
            saved.password().unwrap().as_str(),
            "仅供测试的独立备份密码内容"
        );
        let mut changed = draft(None);
        changed.provider = BackupProviderKind::Webdav;
        changed.endpoint = "https://example.test/backups/".into();
        changed.username = "test".into();
        changed.secret = Some("新的认证秘密占位".into());
        let changed = merge_config(Some(saved), Some(changed)).unwrap();
        assert!(changed.password().is_err());
        assert!(config_setting(&changed).is_err());
    }

    #[test]
    fn configured_password_uses_unicode_character_and_utf8_byte_limits() {
        for password in ["", "short", &"密".repeat(400)] {
            assert!(
                merge_config(
                    Some(decode_setting(1, legacy_value()).unwrap()),
                    Some(draft(Some(password)))
                )
                .and_then(|config| config_setting(&config))
                .is_err()
            );
        }
        assert!(decode_setting(3, serde_json::json!({})).is_err());
    }

    fn prepared(
        runtime: &BackupRuntime,
        owner: &str,
        generation: u64,
    ) -> Result<BackupRestorePreview> {
        runtime.prepare(
            owner.into(),
            generation,
            false,
            BackupConfiguration {
                target: BackupTarget {
                    provider: BackupProviderKind::Gist,
                    endpoint: None,
                    username: None,
                    secret: "仅测试使用的占位内容".into(),
                    device_name: "测试设备".into(),
                },
                backup_password: "仅测试使用的备份密码占位内容".into(),
            },
            OpenedBackup {
                metadata: BackupMetadata {
                    backup_id: uuid::Uuid::new_v4().to_string(),
                    created_at: 1,
                    device_name: "测试设备".into(),
                    app_version: "0.1.0".into(),
                    counts: BackupCounts::default(),
                },
                database: Zeroizing::new(vec![]),
            },
        )
    }

    #[test]
    fn restore_token_checks_window_mode_expiry_and_single_use() {
        let runtime = BackupRuntime::default();
        let preview = prepared(&runtime, "settings", runtime.owner_generation("settings")).unwrap();
        assert!(runtime.take("main", &preview.token, false).is_err());
        assert!(runtime.take("settings", &preview.token, true).is_err());
        assert!(runtime.take("settings", &preview.token, false).is_ok());
        assert!(runtime.take("settings", &preview.token, false).is_err());
        let preview = prepared(&runtime, "settings", runtime.owner_generation("settings")).unwrap();
        runtime
            .prepared
            .lock()
            .unwrap()
            .get_mut(&preview.token)
            .unwrap()
            .created = Instant::now() - PREVIEW_LIFETIME;
        assert!(runtime.take("settings", &preview.token, false).is_err());
    }

    #[test]
    fn closed_window_rejects_late_preview_and_completed_retry_is_owner_bound() {
        let runtime = BackupRuntime::default();
        let generation = runtime.owner_generation("settings");
        runtime.clear_owner("settings");
        assert!(prepared(&runtime, "settings", generation).is_err());
        assert!(runtime.prepared.lock().unwrap().is_empty());
        assert!(prepared(&runtime, "settings", runtime.owner_generation("settings")).is_ok());
        runtime.finish(
            "settings".into(),
            "completed-token".into(),
            BackupImportSummary {
                counts: BackupCounts::default(),
                warnings: vec![],
            },
        );
        assert!(runtime.completed("settings", "completed-token").is_some());
        assert!(runtime.completed("main", "completed-token").is_none());
    }
}

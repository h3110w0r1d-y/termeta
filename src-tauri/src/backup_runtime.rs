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
    pub database: Zeroizing<Vec<u8>>,
    created: Instant,
    pub generation: u64,
}

struct CompletedRestore {
    owner: String,
    created: Instant,
    summary: BackupImportSummary,
    generation: u64,
}

pub struct BackupRuntime {
    operation: Arc<Semaphore>,
    owners: Mutex<HashMap<String, u64>>,
    prepared: Mutex<HashMap<String, PreparedRestore>>,
    completed: Mutex<HashMap<String, CompletedRestore>>,
}

impl Default for BackupRuntime {
    fn default() -> Self {
        Self {
            operation: Arc::new(Semaphore::new(1)),
            owners: Mutex::new(HashMap::new()),
            prepared: Mutex::new(HashMap::new()),
            completed: Mutex::new(HashMap::new()),
        }
    }
}

impl BackupRuntime {
    /// 配置保存持有操作许可，清除全部窗口旧配置对应的预览和重试摘要。
    pub fn clear_config(&self) {
        let mut owners = self.owners.lock().unwrap();
        for generation in owners.values_mut() {
            *generation = generation.wrapping_add(1);
        }
        self.prepared.lock().unwrap().clear();
        self.completed.lock().unwrap().clear();
    }

    pub fn begin(&self) -> Result<OwnedSemaphorePermit> {
        self.operation
            .clone()
            .try_acquire_owned()
            .map_err(|_| failure("另一项备份操作正在进行，请等待完成后重试"))
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
        self.completed
            .lock()
            .unwrap()
            .retain(|_, value| value.owner != owner);
    }

    pub fn prepare(
        &self,
        owner: String,
        generation: u64,
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
                database: opened.database,
                created: Instant::now(),
                generation,
            },
        );
        Ok(BackupRestorePreview { token, metadata })
    }

    pub fn take(&self, owner: &str, token: &str) -> Result<PreparedRestore> {
        self.cleanup();
        let owners = self.owners.lock().unwrap();
        let mut prepared = self.prepared.lock().unwrap();
        let entry = prepared
            .get(token)
            .ok_or_else(|| failure("恢复预览已失效，请重新选择云端备份"))?;
        if entry.owner != owner {
            return Err(failure("此窗口无权提交该恢复预览"));
        }
        if entry.generation != owners.get(owner).copied().unwrap_or_default() {
            prepared.remove(token);
            return Err(failure("恢复窗口或配置已变化，请重新选择云端备份"));
        }
        prepared
            .remove(token)
            .ok_or_else(|| failure("恢复预览已失效"))
    }

    pub fn completed(&self, owner: &str, token: &str) -> Option<BackupImportSummary> {
        self.cleanup();
        let owners = self.owners.lock().unwrap();
        self.completed
            .lock()
            .unwrap()
            .get(token)
            .filter(|entry| {
                entry.owner == owner
                    && entry.generation == owners.get(owner).copied().unwrap_or_default()
            })
            .map(|entry| entry.summary.clone())
    }

    pub fn finish(
        &self,
        owner: String,
        token: String,
        generation: u64,
        summary: BackupImportSummary,
    ) {
        self.cleanup();
        let owners = self.owners.lock().unwrap();
        if owners.get(&owner).copied().unwrap_or_default() != generation {
            return;
        }
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
                generation,
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
) -> Result<BackupConfiguration> {
    merge_config(saved_config(state).await?, draft)
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

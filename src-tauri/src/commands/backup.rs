use crate::{
    backup,
    backup_runtime::{
        BackupConfigDraft, BackupConfigView, BackupListEntry, BackupRestorePreview, config_setting,
        failure, resolve_config, saved_config,
    },
    state::{AppState, settings_owner, storage_error, window_owner},
};
use tauri::{Emitter, Manager, State, WebviewWindow};
use terminal_core::Result;
use terminal_security::{VaultStatus, backup::BackupMetadata};
use terminal_storage::{BackupImportMode, BackupImportSummary};
use zeroize::Zeroizing;

async fn access(window: &WebviewWindow, state: &AppState, initializing: bool) -> Result<String> {
    if initializing {
        let owner = window_owner(window)?;
        if state
            .vault
            .status()
            .await
            .map_err(super::data::security_error)?
            .mode
            != "setup"
        {
            return Err(failure("仅全新保险库可以从初始化页面读取云备份"));
        }
        Ok(owner)
    } else {
        settings_owner(window)?;
        Ok(window.label().to_owned())
    }
}

#[tauri::command]
pub async fn get_backup_config(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<BackupConfigView>> {
    settings_owner(&window)?;
    Ok(saved_config(&state)
        .await?
        .as_ref()
        .map(BackupConfigView::from))
}

#[tauri::command]
pub async fn save_backup_config(
    window: WebviewWindow,
    state: State<'_, AppState>,
    config: BackupConfigDraft,
) -> Result<BackupConfigView> {
    settings_owner(&window)?;
    let _operation = state.backup.begin()?;
    let config = resolve_config(&state, Some(config), false).await?;
    let setting = config_setting(&config)?;
    state
        .store
        .set_setting(&setting.key, setting.version, &setting.value)
        .await
        .map_err(storage_error)?;
    state.backup.clear_owner(window.label());
    let _ = window.app_handle().emit("backup-config-changed", ());
    Ok(BackupConfigView::from(&config))
}

#[tauri::command]
pub async fn test_backup_config(
    window: WebviewWindow,
    state: State<'_, AppState>,
    config: BackupConfigDraft,
    initializing: Option<bool>,
) -> Result<()> {
    let initializing = initializing.unwrap_or(false);
    access(&window, &state, initializing).await?;
    let _operation = state.backup.begin()?;
    let config = resolve_config(&state, Some(config), initializing).await?;
    backup::provider(&config.target)
        .map_err(|error| failure(&error))?
        .test()
        .await
        .map_err(|error| failure(&error))
}

#[tauri::command]
pub async fn list_cloud_backups(
    window: WebviewWindow,
    state: State<'_, AppState>,
    config: Option<BackupConfigDraft>,
    initializing: Option<bool>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<Vec<BackupListEntry>> {
    let initializing = initializing.unwrap_or(false);
    access(&window, &state, initializing).await?;
    let _operation = state.backup.begin()?;
    let config = resolve_config(&state, config, initializing).await?;
    let password = config.password()?;
    let provider = backup::provider(&config.target).map_err(|error| failure(&error))?;
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(10).clamp(1, 20);
    if offset > 1000 {
        return Err(failure("备份列表页码超出限制"));
    }
    let remote = provider.list().await.map_err(|error| failure(&error))?;
    let mut result = Vec::new();
    for entry in remote.into_iter().skip(offset).take(limit) {
        let metadata = match provider.download(&entry.id).await {
            Ok(package) => {
                let password = password.clone();
                tokio::task::spawn_blocking(move || {
                    let package = std::str::from_utf8(&package).map_err(|_| ())?;
                    terminal_security::backup::open(&password, package)
                        .map(|opened| opened.metadata)
                        .map_err(|_| ())
                })
                .await
                .ok()
                .and_then(std::result::Result::ok)
            }
            Err(_) => None,
        };
        result.push(BackupListEntry {
            id: entry.id,
            size: entry.size,
            error: metadata
                .is_none()
                .then(|| "无法解密或读取该备份，请检查密码、网络或格式版本".into()),
            metadata,
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn create_cloud_backup(
    window: WebviewWindow,
    state: State<'_, AppState>,
    device_name: Option<String>,
) -> Result<BackupListEntry> {
    settings_owner(&window)?;
    let _operation = state.backup.begin()?;
    let mut config = resolve_config(&state, None, false).await?;
    let password = config.password()?;
    if let Some(name) = device_name {
        config.target.device_name = name;
    }
    backup::validate_target(&config.target).map_err(|error| failure(&error))?;
    let provider = backup::provider(&config.target).map_err(|error| failure(&error))?;
    let database = state
        .vault
        .export_backup_database()
        .await
        .map_err(super::data::security_error)?;
    let device_name = config.target.device_name.clone();
    let (package, metadata) = tokio::task::spawn_blocking(move || {
        let counts =
            terminal_storage::inspect_backup_database(database.to_vec()).map_err(storage_error)?;
        let metadata = BackupMetadata {
            backup_id: uuid::Uuid::new_v4().to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| failure("系统时间无效"))?
                .as_millis() as u64,
            device_name,
            app_version: env!("CARGO_PKG_VERSION").into(),
            counts,
        };
        let package = terminal_security::backup::seal(&password, &metadata, &database)
            .map_err(super::data::security_error)?;
        Ok::<_, terminal_core::SessionError>((package, metadata))
    })
    .await
    .map_err(|_| failure("备份加密任务中断"))??;
    let remote = provider
        .upload(package.as_bytes())
        .await
        .map_err(|error| failure(&error))?;
    Ok(BackupListEntry {
        id: remote.id,
        size: remote.size,
        metadata: Some(metadata),
        error: None,
    })
}

#[tauri::command]
pub async fn prepare_cloud_restore(
    window: WebviewWindow,
    state: State<'_, AppState>,
    config: Option<BackupConfigDraft>,
    backup_id: String,
    initializing: Option<bool>,
) -> Result<BackupRestorePreview> {
    let initializing = initializing.unwrap_or(false);
    // 在首次异步等待前捕获窗口代次，销毁期间的迟到结果不能重新登记明文。
    let generation = state.backup.owner_generation(window.label());
    let owner = access(&window, &state, initializing).await?;
    let _operation = state.backup.begin()?;
    let config = resolve_config(&state, config, initializing).await?;
    let password = config.password()?;
    let provider = backup::provider(&config.target).map_err(|error| failure(&error))?;
    let package = provider
        .download(&backup_id)
        .await
        .map_err(|error| failure(&error))?;
    let opened = tokio::task::spawn_blocking(move || {
        let package = std::str::from_utf8(&package).map_err(|_| failure("备份包格式无效"))?;
        terminal_security::backup::open(&password, package).map_err(super::data::security_error)
    })
    .await
    .map_err(|_| failure("备份验证任务中断"))??;
    state
        .backup
        .prepare(owner, generation, initializing, config, opened)
}

#[tauri::command]
pub async fn apply_cloud_restore(
    window: WebviewWindow,
    token: String,
) -> Result<BackupImportSummary> {
    settings_owner(&window)?;
    let owner = window.label().to_owned();
    let app = window.app_handle().clone();
    // 确认后的事务独立完成，窗口关闭或接收端消失不能造成重复提交。
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let _operation = state.backup.begin()?;
        if let Some(summary) = state.backup.completed(&owner, &token) {
            return Ok(summary);
        }
        let mut prepared = state.backup.take(&owner, &token, false)?;
        let summary = state
            .vault
            .import_backup_database(
                std::mem::take(&mut *prepared.database),
                BackupImportMode::Merge,
            )
            .await
            .map_err(super::data::security_error)?;
        state.backup.finish(owner, token, summary.clone());
        let _ = app.emit("backup-restored", ());
        Ok(summary)
    })
    .await
    .map_err(|_| failure("恢复任务中断，请重新读取本机数据后核验"))?
}

#[tauri::command]
pub async fn initialize_from_cloud_backup(
    window: WebviewWindow,
    token: String,
    password: String,
) -> Result<VaultStatus> {
    let password = Zeroizing::new(password);
    if password.chars().count() < 12 || password.len() > 1024 {
        return Err(failure("保险库密码至少12个字符，且不超过1024字节"));
    }
    let owner = window_owner(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let _operation = state.backup.begin()?;
        let _initialization = state.backup.initialization.lock().await;
        if state.backup.completed(&owner, &token).is_some() {
            return state.vault.status().await.map_err(super::data::security_error);
        }
        if state.vault.status().await.map_err(super::data::security_error)?.mode != "setup" {
            return Err(failure("本机保险库已经存在，请解锁后使用同步下载"));
        }
        let mut prepared = state.backup.take(&owner, &token, true)?;
        let setting = config_setting(&prepared.config)?;
        state.vault.unlock_with_password(password, true).await.map_err(super::data::security_error)?;
        let summary = state.vault.import_backup_database_with_setting(std::mem::take(&mut *prepared.database), BackupImportMode::Merge, Some(setting)).await.map_err(|_| failure("本地保险库已创建，但云端数据导入未完成；请进入保险库后重新配置并同步，原云备份保持不变"))?;
        state.backup.finish(owner, token, summary);
        let _ = app.emit("backup-restored", ());
        let _ = app.emit("backup-config-changed", ());
        state.vault.status().await.map_err(super::data::security_error)
    }).await.map_err(|_| failure("云端初始化任务中断，请检查保险库状态后重试"))?
}

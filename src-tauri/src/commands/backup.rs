use crate::{
    account::runtime::OfflineOperation,
    backup,
    backup_runtime::{
        BackupConfigDraft, BackupConfigView, BackupListEntry, BackupRestorePreview, config_setting,
        failure, resolve_config, saved_config,
    },
    state::{AppState, settings_owner, storage_error},
};
use tauri::{Emitter, Manager, State, WebviewWindow};
use terminal_core::Result;
use terminal_security::backup::BackupMetadata;
use terminal_storage::{BackupImportMode, BackupImportSummary};

async fn require_offline(state: &AppState) -> Result<()> {
    if crate::account::service::status(state).await?.user.is_some() {
        return Err(failure("官方模式自动同步，请先退出账号再使用第三方备份"));
    }
    Ok(())
}

async fn access(window: &WebviewWindow, state: &AppState) -> Result<OfflineOperation> {
    settings_owner(window)?;
    // 先登记再读取账号，切换必须等待这份租约，不能越过检查后的网络/事务操作。
    let operation = state.account.begin_offline()?;
    require_offline(state).await?;
    Ok(operation)
}

#[tauri::command]
pub async fn get_backup_config(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<BackupConfigView>> {
    let _mode = access(&window, &state).await?;
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
    let _mode = access(&window, &state).await?;
    let _operation = state.backup.begin()?;
    let config = resolve_config(&state, Some(config)).await?;
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
) -> Result<()> {
    let _mode = access(&window, &state).await?;
    let _operation = state.backup.begin()?;
    let config = resolve_config(&state, Some(config)).await?;
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
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<Vec<BackupListEntry>> {
    let _mode = access(&window, &state).await?;
    let _operation = state.backup.begin()?;
    let config = resolve_config(&state, None).await?;
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
    let _mode = access(&window, &state).await?;
    let _operation = state.backup.begin()?;
    let mut config = resolve_config(&state, None).await?;
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
    backup_id: String,
) -> Result<BackupRestorePreview> {
    // 在首次异步等待前捕获窗口代次，销毁期间的迟到结果不能重新登记明文。
    let generation = state.backup.owner_generation(window.label());
    let mode = access(&window, &state).await?;
    let _operation = state.backup.begin()?;
    let config = resolve_config(&state, None).await?;
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
        .prepare(window.label().into(), generation, mode.generation, opened)
}

#[tauri::command]
pub async fn apply_cloud_restore(
    window: WebviewWindow,
    token: String,
) -> Result<BackupImportSummary> {
    let state = window.state::<AppState>();
    let mode = access(&window, &state).await?;
    let owner = window.label().to_owned();
    let operation = state.backup.begin()?;
    if let Some(summary) = state.backup.completed(&owner, &token, mode.generation) {
        return Ok(summary);
    }
    // 消费令牌与登记操作在独立任务创建前完成，已确认事务不依赖窗口继续存活。
    let mut prepared = state.backup.take(&owner, &token, mode.generation)?;
    let app = window.app_handle().clone();
    // 确认后的事务独立完成，窗口关闭或接收端消失不能造成重复提交。
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let _operation = operation;
        // 整份租约移入任务，避免闭包只捕获 generation 字段而提前释放操作期。
        let mode_lease = mode;
        let summary = state
            .vault
            .import_backup_database(
                std::mem::take(&mut *prepared.database),
                BackupImportMode::Merge,
            )
            .await
            .map_err(super::data::security_error)?;
        state
            .backup
            .finish(owner, token, mode_lease.generation, summary.clone());
        let _ = app.emit("backup-restored", ());
        Ok(summary)
    })
    .await
    .map_err(|_| failure("恢复任务中断，请重新读取本机数据后核验"))?
}

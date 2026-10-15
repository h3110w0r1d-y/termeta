use crate::state::{AppState, owner, storage_error};
use tauri::{State, WebviewWindow};
use terminal_core::{Result, SessionError};
use terminal_storage::{CredentialRecord, GroupDraft, HostDraft, HostGroup, HostProfile};

#[tauri::command]
pub async fn list_hosts(
    window: WebviewWindow,
    state: State<'_, AppState>,
    search: Option<String>,
) -> Result<Vec<HostProfile>> {
    owner(&window)?;
    let search = search.unwrap_or_default();
    if search.len() > 256 {
        return Err(SessionError::new(
            "INVALID_CONFIG",
            "validation",
            "搜索文本过长",
        ));
    }
    state.store.list_hosts(&search).await.map_err(storage_error)
}
#[tauri::command]
pub async fn save_host(
    window: WebviewWindow,
    state: State<'_, AppState>,
    mut host: HostDraft,
    expected_revision: Option<i64>,
) -> Result<HostProfile> {
    owner(&window)?;
    host.revision = expected_revision;
    state.store.save_host(host).await.map_err(storage_error)
}
#[tauri::command]
pub async fn delete_host(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
    revision: i64,
) -> Result<()> {
    owner(&window)?;
    state
        .store
        .delete_host(&id, revision)
        .await
        .map_err(storage_error)
}
#[tauri::command]
pub async fn list_host_groups(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<HostGroup>> {
    owner(&window)?;
    state.store.list_groups().await.map_err(storage_error)
}
#[tauri::command]
pub async fn save_host_group(
    window: WebviewWindow,
    state: State<'_, AppState>,
    mut group: GroupDraft,
    expected_revision: Option<i64>,
) -> Result<HostGroup> {
    owner(&window)?;
    group.revision = expected_revision;
    state.store.save_group(group).await.map_err(storage_error)
}
#[tauri::command]
pub async fn delete_host_group(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
    revision: i64,
) -> Result<()> {
    owner(&window)?;
    state
        .store
        .delete_group(&id, revision)
        .await
        .map_err(storage_error)
}
#[tauri::command]
pub async fn list_known_hosts(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>> {
    owner(&window)?;
    Ok(state.store.list_known_hosts().await.map_err(storage_error)?.into_iter().map(|key| serde_json::json!({
        "id":key.id,"canonicalHost":key.canonical_host,"port":key.port,"keyAlgorithm":key.key_algorithm,
        "fingerprintSha256":key.fingerprint_sha256,"trustedAt":key.trusted_at,
    })).collect())
}
#[tauri::command]
pub async fn revoke_known_host(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<()> {
    owner(&window)?;
    state.store.revoke_key(&id).await.map_err(storage_error)
}
#[tauri::command]
pub async fn list_credentials(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<CredentialRecord>> {
    owner(&window)?;
    state.store.list_credentials().await.map_err(storage_error)
}
#[tauri::command]
pub async fn save_credential(
    window: WebviewWindow,
    state: State<'_, AppState>,
    kind: String,
    label: String,
    secret: String,
) -> Result<serde_json::Value> {
    let secret = zeroize::Zeroizing::new(secret);
    owner(&window)?;
    if !matches!(kind.as_str(), "password" | "passphrase")
        || label.is_empty()
        || label.len() > 128
        || label.chars().any(char::is_control)
    {
        return Err(SessionError::new(
            "INVALID_CONFIG",
            "validation",
            "凭据类型或标签无效",
        ));
    }
    let id = state
        .vault
        .save(&kind, &label, secret)
        .await
        .map_err(security_error)?;
    Ok(serde_json::json!({"id":id}))
}
#[tauri::command]
pub async fn delete_credential(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<()> {
    owner(&window)?;
    state.vault.delete(&id).await.map_err(security_error)
}
#[tauri::command]
pub async fn get_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>> {
    owner(&window)?;
    let Some(setting) = state
        .store
        .get_setting("app")
        .await
        .map_err(storage_error)?
    else {
        return Ok(None);
    };
    if setting.value_version != 1 {
        return Err(SessionError::new(
            "UNSUPPORTED_SETTINGS_VERSION",
            "storage",
            "设置来自不支持的版本，已停止写入",
        ));
    }
    crate::settings::validate(&setting.value)?;
    Ok(Some(setting.value))
}
#[tauri::command]
pub async fn save_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
    settings: serde_json::Value,
) -> Result<()> {
    owner(&window)?;
    if let Some(current) = state
        .store
        .get_setting("app")
        .await
        .map_err(storage_error)?
    {
        if current.value_version != 1 || current.value["schemaVersion"] != 1 {
            return Err(SessionError::new(
                "UNSUPPORTED_SETTINGS_VERSION",
                "storage",
                "现有设置来自不支持的版本，拒绝覆盖",
            ));
        }
    }
    crate::settings::validate(&settings)?;
    state
        .store
        .set_setting("app", 1, &settings)
        .await
        .map_err(storage_error)
}
pub fn security_error(error: terminal_security::SecurityError) -> SessionError {
    SessionError::new(
        "CREDENTIAL_UNAVAILABLE",
        "authentication",
        &error.to_string(),
    )
}

#[tauri::command]
pub async fn list_history(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<terminal_storage::HistoryEntry>> {
    owner(&window)?;
    state.store.list_history().await.map_err(storage_error)
}
#[tauri::command]
pub async fn clear_history(window: WebviewWindow, state: State<'_, AppState>) -> Result<()> {
    owner(&window)?;
    state.store.clear_history().await.map_err(storage_error)
}
#[tauri::command]
pub async fn recover_credential(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<()> {
    owner(&window)?;
    state
        .vault
        .recover_pending(&id)
        .await
        .map_err(security_error)
}

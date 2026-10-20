use crate::state::{AppState, owner, storage_error};
use tauri::{Emitter, Manager, State, WebviewWindow};
use terminal_core::{Result, SessionError};
use terminal_storage::{CredentialRecord, GroupDraft, HostDraft, HostGroup, HostProfile};

#[tauri::command]
pub async fn list_host_inventory(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<terminal_storage::HostInventory> {
    owner(&window)?;
    state
        .store
        .list_host_inventory()
        .await
        .map_err(storage_error)
}

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
    password: Option<String>,
) -> Result<HostProfile> {
    let password = password.map(zeroize::Zeroizing::new);
    owner(&window)?;
    host.revision = expected_revision;
    state
        .vault
        .save_host(host, password)
        .await
        .map_err(security_error)
}
#[tauri::command]
pub async fn save_workspace(
    window: WebviewWindow,
    state: State<'_, AppState>,
    draft: terminal_storage::WorkspaceDraft,
) -> Result<HostProfile> {
    owner(&window)?;
    state
        .store
        .save_workspace(draft)
        .await
        .map_err(storage_error)
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
        .map_err(storage_error)?;
    // 主机删除已提交；孤立秘密保留待清理状态，后续解锁重试回收。
    let _ = state.vault.reconcile_deletions().await;
    Ok(())
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
    delete_hosts: Option<bool>,
) -> Result<()> {
    owner(&window)?;
    if delete_hosts.unwrap_or(false) {
        state
            .store
            .delete_group_and_hosts(&id, revision)
            .await
            .map_err(storage_error)?;
        // 删除已提交，专属秘密清理失败由后续解锁继续回收，公共凭据不受影响。
        let _ = state.vault.reconcile_deletions().await;
        Ok(())
    } else {
        state
            .store
            .delete_group(&id, revision)
            .await
            .map_err(storage_error)
    }
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
    password: Option<String>,
    key_file_ref: Option<String>,
    passphrase: Option<String>,
) -> Result<serde_json::Value> {
    let password = password.map(zeroize::Zeroizing::new);
    let passphrase = passphrase
        .map(zeroize::Zeroizing::new)
        .filter(|value| !value.is_empty());
    owner(&window)?;
    if !matches!(kind.as_str(), "password" | "privateKey")
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
    let id = match kind.as_str() {
        "password" if key_file_ref.is_none() && passphrase.is_none() => {
            state
                .vault
                .save_password(&label, password.unwrap_or_default())
                .await
        }
        "privateKey" if password.is_none() => {
            let key_file_ref = key_file_ref.ok_or_else(|| {
                SessionError::new("INVALID_CONFIG", "validation", "缺少授权私钥引用")
            })?;
            state
                .vault
                .save_private_key(&label, &key_file_ref, passphrase)
                .await
        }
        _ => {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "validation",
                "凭据内容与类型不匹配",
            ));
        }
    }
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

pub fn security_error(error: terminal_security::SecurityError) -> SessionError {
    if let terminal_security::SecurityError::Metadata(error) = error {
        return storage_error(error);
    }
    SessionError::new(
        "CREDENTIAL_UNAVAILABLE",
        "authentication",
        &error.to_string(),
    )
}

#[tauri::command]
pub async fn vault_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<terminal_security::VaultStatus> {
    crate::state::window_owner(&window)?;
    state.vault.status().await.map_err(security_error)
}
#[tauri::command]
pub async fn unlock_vault(
    window: WebviewWindow,
    state: State<'_, AppState>,
    password: String,
    setup: bool,
) -> Result<terminal_security::VaultStatus> {
    let password = zeroize::Zeroizing::new(password);
    crate::state::window_owner(&window)?;
    state
        .vault
        .unlock_with_password(password, setup)
        .await
        .map_err(security_error)
}

#[tauri::command]
pub async fn change_vault_password(
    window: WebviewWindow,
    old_password: String,
    new_password: String,
) -> Result<terminal_security::PasswordChangeResult> {
    let old_password = zeroize::Zeroizing::new(old_password);
    let new_password = zeroize::Zeroizing::new(new_password);
    crate::state::settings_owner(&window)?;
    let handle = window.app_handle().clone();
    // 独立任务持有本地改密操作，窗口关闭不会取消已开始的文件切换。
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<AppState>();
        state
            .vault
            .change_password(old_password, new_password)
            .await
            .map_err(security_error)
    })
    .await
    .map_err(|_| {
        SessionError::new(
            "VAULT_CHANGE_INTERRUPTED",
            "storage",
            "密码修改任务中断，请退出应用后重试并保留保险库文件",
        )
    })?
}

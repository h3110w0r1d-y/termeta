//! 同步命令仅暴露状态和密码输入，包正文及主密钥不经过 WebView。
use crate::{
    official_sync::{self, Status},
    state::{AppState, account_owner, window_owner},
};
use tauri::{Emitter, Manager, State, WebviewWindow};
use terminal_core::Result;

#[tauri::command]
pub async fn get_official_sync_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Status> {
    account_owner(&window)?;
    let result = official_sync::status(&state).await;
    if state
        .official
        .changed
        .swap(false, std::sync::atomic::Ordering::AcqRel)
    {
        let _ = window.app_handle().emit("backup-restored", ());
    }
    result
}
#[tauri::command]
pub async fn complete_official_sync(
    window: WebviewWindow,
    state: State<'_, AppState>,
    password: String,
) -> Result<Status> {
    let password = zeroize::Zeroizing::new(password);
    window_owner(&window)?;
    let result = official_sync::complete(&state, password).await;
    let _ = window.app_handle().emit("official-sync-changed", ());
    let _ = window.app_handle().emit("account-changed", ());
    let _ = window.app_handle().emit("backup-restored", ());
    result
}
#[tauri::command]
pub async fn sync_official_vault(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Status> {
    account_owner(&window)?;
    let result = official_sync::sync(&state).await;
    let _ = window.app_handle().emit("official-sync-changed", ());
    let _ = window.app_handle().emit("backup-restored", ());
    result
}

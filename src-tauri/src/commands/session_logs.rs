use crate::state::{AppState, owner, storage_error};
use tauri::{State, WebviewWindow};
use terminal_core::Result;
use terminal_storage::{SessionLog, SessionLogDraft, SessionLogSummary};

#[tauri::command]
pub async fn save_session_log(
    window: WebviewWindow,
    state: State<'_, AppState>,
    draft: SessionLogDraft,
) -> Result<SessionLogSummary> {
    owner(&window)?;
    state
        .store
        .save_session_log(draft)
        .await
        .map_err(storage_error)
}

#[tauri::command]
pub async fn list_session_logs(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<SessionLogSummary>> {
    owner(&window)?;
    state.store.list_session_logs().await.map_err(storage_error)
}

#[tauri::command]
pub async fn read_session_log(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<SessionLog> {
    owner(&window)?;
    state
        .store
        .read_session_log(&id)
        .await
        .map_err(storage_error)
}

#[tauri::command]
pub async fn delete_session_log(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<()> {
    owner(&window)?;
    state
        .store
        .delete_session_log(&id)
        .await
        .map_err(storage_error)
}

#[tauri::command]
pub async fn clear_session_logs(window: WebviewWindow, state: State<'_, AppState>) -> Result<()> {
    owner(&window)?;
    state
        .store
        .clear_session_logs()
        .await
        .map_err(storage_error)
}

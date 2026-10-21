use crate::files::{browser, connected};
use crate::state::{AppState, owner};
use serde::Deserialize;
use tauri::{State, WebviewWindow};
use terminal_core::Result;
use terminal_files::file_error;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SftpEntryRequest {
    pub session_id: String,
    pub path: String,
    pub names: Vec<String>,
}

#[tauri::command]
pub async fn sftp_delete(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: SftpEntryRequest,
    recursive: bool,
) -> Result<()> {
    let owner_id = owner(&window)?;
    let browser = browser(&state, &owner_id, &request.session_id).await?;
    let _operation = browser
        .operation
        .try_lock()
        .map_err(|_| file_error("文件操作正在进行，请稍后重试"))?;
    let _budget = state
        .transfers
        .clone()
        .try_acquire_owned()
        .map_err(|_| file_error("文件操作队列已满，请稍后重试"))?;
    connected(&state, &owner_id, &request.session_id)?;
    browser
        .remove_entries(
            &request.path,
            &request.names,
            recursive,
            &browser.transfer_cancellation(),
        )
        .await
}

#[tauri::command]
pub async fn sftp_set_permissions(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: SftpEntryRequest,
    permissions: u32,
) -> Result<()> {
    let owner_id = owner(&window)?;
    let browser = browser(&state, &owner_id, &request.session_id).await?;
    let _operation = browser
        .operation
        .try_lock()
        .map_err(|_| file_error("文件操作正在进行，请稍后重试"))?;
    let _budget = state
        .transfers
        .clone()
        .try_acquire_owned()
        .map_err(|_| file_error("文件操作队列已满，请稍后重试"))?;
    connected(&state, &owner_id, &request.session_id)?;
    browser
        .set_entry_permissions(
            &request.path,
            &request.names,
            permissions,
            &browser.transfer_cancellation(),
        )
        .await
}

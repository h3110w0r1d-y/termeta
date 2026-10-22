use crate::files::{TransferRegistration, browser, connected, download_to_path, upload_paths};
use crate::state::{AppState, owner};
use std::path::PathBuf;
use tauri::{Manager, State, WebviewWindow, ipc::Channel};
use tauri_plugin_dialog::DialogExt;
use terminal_core::{Result, SessionError};
use terminal_files::{
    DirectoryPage, FileProvider, TransferProgress, child_path, file_error, validate_name,
};

#[tauri::command]
pub async fn sftp_list(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    more: bool,
) -> Result<DirectoryPage> {
    let browser = browser(&state, &owner(&window)?, &session_id).await?;
    let _operation = browser
        .operation
        .try_lock()
        .map_err(|_| file_error("文件操作正在进行，请稍后重试"))?;
    let result = browser.list(&path, more).await;
    if result.is_err() && !more {
        // 刷新失败后允许下次重新申请子系统；不重建或断开终端。
        state.sftp.lock().unwrap().remove(&session_id);
    }
    result
}
#[tauri::command]
pub fn sftp_cancel(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    transfer_id: String,
) -> Result<()> {
    let owner = owner(&window)?;
    state
        .session_manager(&owner, &session_id)?
        .get(&owner, &session_id)?;
    if let Some(browser) = state.sftp.lock().unwrap().get(&session_id) {
        browser.cancel_transfer(&transfer_id);
    }
    Ok(())
}
#[tauri::command]
pub async fn sftp_drop_upload(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    transfer_id: String,
    drop_id: String,
    directory: String,
    on_progress: Channel<TransferProgress>,
) -> Result<bool> {
    let owner_id = owner(&window)?;
    let paths = window
        .state::<crate::file_drag::LocalDropRegistry>()
        .take(&owner_id, &drop_id)
        .map_err(|message| file_error(&message))?;
    transfer_paths(
        &window,
        &state,
        &owner_id,
        &session_id,
        transfer_id,
        directory,
        None,
        Some(paths),
        &on_progress,
    )
    .await
}

async fn transfer_paths(
    window: &WebviewWindow,
    state: &AppState,
    owner_id: &str,
    session_id: &str,
    transfer_id: String,
    directory: String,
    name: Option<String>,
    paths: Option<Vec<PathBuf>>,
    on_progress: &Channel<TransferProgress>,
) -> Result<bool> {
    if uuid::Uuid::parse_str(&transfer_id).is_err() {
        return Err(file_error("传输标识必须为 UUID"));
    }
    let browser = browser(state, owner_id, session_id).await?;
    let _operation = browser
        .operation
        .try_lock()
        .map_err(|_| file_error("文件操作正在进行，请稍后重试"))?;
    let _budget = state
        .transfers
        .clone()
        .try_acquire_owned()
        .map_err(|_| file_error("传输队列已满，请稍后重试"))?;
    let cancel = browser.transfer_cancellation();
    let _transfer = TransferRegistration::new(&transfer_id, cancel.clone(), [browser.clone()]);
    if on_progress
        .send(TransferProgress {
            transferred: "0".into(),
            total: None,
            current_name: Some("准备上传…".into()),
            completed_files: Some(0),
            total_files: None,
        })
        .is_err()
    {
        cancel.cancel();
    }
    let paths = if let Some(paths) = paths {
        paths
    } else {
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .dialog()
            .file()
            .set_parent(window)
            .set_title("选择要上传的文件（可多选）")
            .pick_files(move |files| {
                let _ = tx.send(files);
            });
        let files = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(SessionError::cancelled()),
            result = rx => result.map_err(|_| file_error("文件选择器已关闭"))?,
        };
        let Some(files) = files else {
            return Ok(false);
        };
        files
            .into_iter()
            .map(|file| {
                file.into_path()
                    .map_err(|_| file_error("请选择本地文件路径"))
            })
            .collect::<Result<Vec<_>>>()?
    };
    connected(state, owner_id, session_id)?;
    upload_paths(
        window,
        state,
        owner_id,
        session_id,
        &browser,
        paths,
        &directory,
        name,
        &cancel,
        on_progress,
    )
    .await?;
    Ok(true)
}

#[tauri::command]
pub async fn sftp_transfer(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    transfer_id: String,
    directory: String,
    name: Option<String>,
    upload: bool,
    on_progress: Channel<TransferProgress>,
) -> Result<bool> {
    let owner_id = owner(&window)?;
    connected(&state, &owner_id, &session_id)?;
    if let Some(name) = &name {
        validate_name(name)?;
    }
    if upload {
        return transfer_paths(
            &window,
            &state,
            &owner_id,
            &session_id,
            transfer_id,
            directory,
            name,
            None,
            &on_progress,
        )
        .await;
    }
    if uuid::Uuid::parse_str(&transfer_id).is_err() {
        return Err(file_error("传输标识必须为 UUID"));
    }
    let browser = browser(&state, &owner_id, &session_id).await?;
    let _operation = browser
        .operation
        .try_lock()
        .map_err(|_| file_error("文件操作正在进行，请稍后重试"))?;
    let _budget = state
        .transfers
        .clone()
        .try_acquire_owned()
        .map_err(|_| file_error("传输队列已满，请稍后重试"))?;
    let name = name.ok_or_else(|| file_error("请选择下载文件"))?;
    let destination = child_path(&directory, &name)?;
    let cancel = browser.transfer_cancellation();
    let _transfer = TransferRegistration::new(&transfer_id, cancel.clone(), [browser.clone()]);
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("下载到新文件（同名目标将拒绝写入）")
        .set_file_name(&name)
        .save_file(move |file| {
            let _ = tx.send(file);
        });
    let selected = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(SessionError::cancelled()),
        result = rx => result.map_err(|_| file_error("文件选择器已关闭"))?,
    };
    let Some(file) = selected else {
        return Ok(false);
    };
    let local = file
        .into_path()
        .map_err(|_| file_error("请选择本地文件路径"))?;
    connected(&state, &owner_id, &session_id)?;
    download_to_path(&browser, &destination, &local, &cancel, |value| {
        if on_progress.send(value).is_err() {
            cancel.cancel();
        }
    })
    .await?;
    Ok(true)
}

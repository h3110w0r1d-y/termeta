use crate::files::{browser, connected, download_directory_to_path, download_to_path};
use crate::{
    file_drag::PromisedFile,
    state::{AppState, owner},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tauri::{Manager, State, WebviewWindow, ipc::Channel};
use terminal_core::{CancellationToken, Result};
use terminal_files::{TransferProgress, child_path, file_error};
use terminal_sftp::SftpBrowser;

struct DragTransfer {
    browser: Arc<SftpBrowser>,
    id: String,
    cancel: CancellationToken,
}
impl Drop for DragTransfer {
    fn drop(&mut self) {
        // 原生拖放取消、异常或窗口退出后，迟到的文件承诺不得继续读取远端。
        self.cancel.cancel();
        let mut active = self.browser.transfer.lock().unwrap();
        if active.as_ref().is_some_and(|(id, _)| id == &self.id) {
            active.take();
        }
    }
}

#[tauri::command]
pub async fn sftp_start_drag(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    transfer_id: String,
    directory: String,
    names: Vec<String>,
    directories: Vec<String>,
    on_progress: Channel<TransferProgress>,
) -> Result<bool> {
    let owner = owner(&window)?;
    if !cfg!(any(target_os = "macos", target_os = "windows")) {
        return Err(file_error("此平台尚未支持原生文件拖出，请使用下载按钮"));
    }
    if uuid::Uuid::parse_str(&transfer_id).is_err() || names.is_empty() || names.len() > 100 {
        return Err(file_error("拖出任务无效，每次最多100个文件或文件夹"));
    }
    let browser = browser(&state, &owner, &session_id).await?;
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
    browser.register_transfer(&transfer_id, cancel.clone());
    let _transfer = DragTransfer {
        browser: browser.clone(),
        id: transfer_id,
        cancel: cancel.clone(),
    };
    let directories: std::collections::HashSet<_> = directories.into_iter().collect();
    if directories.iter().any(|name| !names.contains(name)) {
        return Err(file_error("文件夹标记与拖出条目不匹配"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut paths = Vec::new();
    for name in &names {
        if !seen.insert(name.clone()) {
            return Err(file_error("拖出列表包含重复文件"));
        }
        let path = child_path(&directory, name)?;
        // 开始原生手势前不等待网络；实际提供内容时由reader核对普通文件类型。
        paths.push(path);
    }
    connected(&state, &owner, &session_id)?;
    #[cfg(target_os = "windows")]
    {
        return crate::file_drag_windows::start_drag(
            window,
            browser.clone(),
            session_id,
            names,
            paths,
            directories,
            cancel.clone(),
            on_progress,
        )
        .await;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let count = names.len();
        let completed = Arc::new(AtomicUsize::new(0));
        let serial = Arc::new(tokio::sync::Mutex::new(()));
        let channel = Arc::new(on_progress);
        let files = names
            .into_iter()
            .zip(paths)
            .map(|(name, remote)| {
                let browser = browser.clone();
                let cancel = cancel.clone();
                let completed = completed.clone();
                let serial = serial.clone();
                let channel = channel.clone();
                let app = window.app_handle().clone();
                let owner = owner.clone();
                let session_id = session_id.clone();
                let directory = directories.contains(&name);
                PromisedFile {
                    directory,
                    name: name.clone(),
                    write: Arc::new(move |local| {
                        tauri::async_runtime::block_on(async {
                            let _serial = serial.lock().await;
                            if cancel.is_cancelled() {
                                return Err("拖出下载已取消".into());
                            }
                            connected(&app.state::<AppState>(), &owner, &session_id)
                                .map_err(|error| error.message)?;
                            // 只有外部目标请求兑现文件承诺，才通知前端出现下载任务。
                            if channel
                                .send(TransferProgress {
                                    transferred: "0".into(),
                                    total: None,
                                    current_name: Some(name.clone()),
                                    completed_files: Some(completed.load(Ordering::SeqCst)),
                                    total_files: Some(count),
                                })
                                .is_err()
                            {
                                cancel.cancel();
                            }
                            let report = |mut progress: TransferProgress| {
                                if !directory {
                                    progress.current_name = Some(name.clone());
                                    progress.completed_files =
                                        Some(completed.load(Ordering::SeqCst));
                                    progress.total_files = Some(count);
                                }
                                if channel.send(progress).is_err() {
                                    cancel.cancel();
                                }
                            };
                            if directory {
                                download_directory_to_path(
                                    &browser, &remote, &local, &cancel, report,
                                )
                                .await
                            } else {
                                download_to_path(&browser, &remote, &local, &cancel, report).await
                            }
                            .map_err(|error| error.message)?;
                            let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
                            let _ = channel.send(TransferProgress {
                                transferred: "0".into(),
                                total: None,
                                current_name: Some(name.clone()),
                                completed_files: Some(done),
                                total_files: Some(count),
                            });
                            Ok(())
                        })
                    }),
                }
            })
            .collect();
        // 命令独占拖出令牌；原生层只接收本次手势，不再从全局表隐式复用。
        let outgoing = window
            .state::<crate::file_drag::LocalDropRegistry>()
            .begin_outgoing()
            .map_err(|error| file_error(&error))?;
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(terminal_core::SessionError::cancelled()),
            _ = outgoing.cancelled() => Ok(false),
            result = crate::file_drag::start_drag(window.clone(), files, outgoing.clone()) => result.map_err(|error| file_error(&error)),
        };
        outgoing.finish();
        cancel.cancel();
        // 取消先阻断迟到承诺，并等待正在运行的回调完成关闭/清理，再释放会话操作锁。
        let _drained = serial.lock().await;
        crate::file_drag::finish_drag(&window, &outgoing)
            .await
            .map_err(|error| file_error(&error))?;
        result
    }
}

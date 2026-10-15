use crate::state::{AppState, owner};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tauri::{Manager, State, WebviewWindow, ipc::Channel};
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};
use terminal_core::{Result, SessionError, SessionState};
use terminal_files::{
    DirectoryPage, FileProvider, TransferProgress, child_path, copy_file, file_error, validate_name,
};
use terminal_sftp::SftpBrowser;

pub(super) fn connected(state: &AppState, owner: &str, id: &str) -> Result<()> {
    if state.sessions.get(owner, id)?.state != SessionState::Connected {
        return Err(file_error("SSH 已断开，文件列表已过期，请重连后刷新"));
    }
    Ok(())
}
pub(super) async fn browser(state: &AppState, owner: &str, id: &str) -> Result<Arc<SftpBrowser>> {
    connected(state, owner, id)?;
    let gate = state.request_gate(&format!("sftp:{id}"));
    let _gate = gate.lock().await;
    let existing = state.sftp.lock().unwrap().get(id).cloned();
    if let Some(browser) = existing {
        return Ok(browser);
    }
    let browser = Arc::new(SftpBrowser::connect(&state.ssh_channels, owner, id).await?);
    connected(state, owner, id)?;
    state
        .sftp
        .lock()
        .unwrap()
        .insert(id.into(), browser.clone());
    Ok(browser)
}
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
    state.sessions.get(&owner(&window)?, &session_id)?;
    if let Some(browser) = state.sftp.lock().unwrap().get(&session_id) {
        browser.cancel_transfer(&transfer_id);
    }
    Ok(())
}
struct TransferGuard(Arc<SftpBrowser>);
impl Drop for TransferGuard {
    fn drop(&mut self) {
        if let Some((_, cancel)) = self.0.transfer.lock().unwrap().take() {
            cancel.cancel();
        }
    }
}
/// 原生拖出与保存对话框共用；调用者负责会话所有权、操作锁和传输许可。
pub async fn download_to_path(
    browser: &SftpBrowser,
    remote: &str,
    local: &Path,
    cancel: &terminal_core::CancellationToken,
    progress: impl Fn(TransferProgress),
) -> Result<()> {
    match tokio::fs::symlink_metadata(&local).await {
        Ok(_) => return Err(file_error("本地同名目标已存在，请选择新名称")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(file_error("无法检查本地目标")),
    }
    let attrs = browser.metadata(remote).await?;
    let mut source = browser.reader(remote).await?;
    // 由本次原生对话框授权精确目标；不接收 WebView 提供的任意本地路径。
    let parent = local.parent().ok_or_else(|| file_error("保存目录无效"))?;
    let temporary = parent.join(format!(".termvexa-{}.part", uuid::Uuid::new_v4()));
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut target = options
        .open(&temporary)
        .await
        .map_err(|_| file_error("无法创建本地临时文件"))?;
    let result = copy_file(&mut source, &mut target, attrs.size, &cancel, progress).await;
    drop(target);
    let result = match result {
        Ok(_) if !cancel.is_cancelled() => tokio::fs::hard_link(&temporary, &local)
            .await
            .map_err(|_| file_error("保存失败：目标已存在或文件系统不支持安全发布，请选择新名称")),
        Ok(_) => Err(SessionError::cancelled()),
        Err(error) => Err(error),
    };
    if tokio::fs::remove_file(&temporary).await.is_err() {
        return Err(file_error(&format!(
            "{}；本地临时文件清理失败：{}",
            result
                .as_ref()
                .err()
                .map(|e| e.message.as_str())
                .unwrap_or("下载已保存"),
            temporary.display()
        )));
    }
    result?;
    Ok(())
}

/// 只写入Finder本次授权的新目录，递归失败保留已完成内容；不跟随远端链接。
pub(super) async fn download_directory_to_path(
    browser: &SftpBrowser,
    remote: &str,
    local: &Path,
    cancel: &terminal_core::CancellationToken,
    progress: impl Fn(TransferProgress),
) -> Result<()> {
    if cancel.is_cancelled() {
        return Err(SessionError::cancelled());
    }
    if !browser.metadata(remote).await?.file_type().is_dir() {
        return Err(file_error("拖出目标不是普通目录，不递归下载软连接"));
    }
    // create_dir本身拒绝所有已存在目标，绝不删除Finder目录后再写入。
    tokio::fs::create_dir(local)
        .await
        .map_err(|_| file_error("下载根目录已存在或无法创建，不替换任何已有目录"))?;
    let mut pending = vec![(remote.to_string(), local.to_path_buf(), 0usize, true)];
    let mut discovered = 1usize;
    let mut completed = 0usize;
    let result: Result<()> = async {
        while let Some((source, destination, depth, root)) = pending.pop() {
            if cancel.is_cancelled() {
                return Err(SessionError::cancelled());
            }
            if depth > 64 {
                return Err(file_error("目录下载超过 64 层限制"));
            }
            let attrs = browser.metadata(&source).await?;
            if attrs.file_type().is_dir() {
                if !root {
                    tokio::fs::create_dir(&destination)
                        .await
                        .map_err(|_| file_error("下载子目录已存在或无法创建，不替换已有目标"))?;
                }
                progress(TransferProgress {
                    transferred: "0".into(),
                    total: None,
                    current_name: Some(source.clone()),
                    completed_files: Some(completed),
                    total_files: None,
                });
                let entries = browser
                    .scan_directory(&source, 10000usize.saturating_sub(discovered), cancel)
                    .await?;
                discovered += entries.len();
                for entry in entries.into_iter().rev() {
                    pending.push((
                        child_path(&source, &entry.name)?,
                        destination.join(&entry.name),
                        depth + 1,
                        false,
                    ));
                }
            } else if attrs.file_type().is_file() && !root {
                download_to_path(browser, &source, &destination, cancel, |mut value| {
                    value.current_name = Some(source.clone());
                    value.completed_files = Some(completed);
                    value.total_files = None;
                    progress(value);
                })
                .await?;
                completed += 1;
            } else {
                return Err(file_error("目录内容已变化，拒绝下载软连接或特殊文件"));
            }
        }
        if cancel.is_cancelled() {
            return Err(SessionError::cancelled());
        }
        progress(TransferProgress {
            transferred: "0".into(),
            total: None,
            current_name: Some(remote.into()),
            completed_files: Some(completed),
            total_files: Some(completed),
        });
        Ok(())
    }
    .await;
    result.map_err(|mut error| {
        error.message = format!(
            "{}；目录下载已完成 {completed} 个文件，已下载内容保留于 {}，未覆盖或删除原有目录",
            error.message,
            local.display()
        );
        error
    })
}

struct UploadItem {
    local: PathBuf,
    remote: String,
    directory: bool,
}

async fn upload_items(
    roots: Vec<PathBuf>,
    directory: &str,
    name: Option<String>,
) -> Result<Vec<UploadItem>> {
    if roots.len() > 1 && name.is_some() {
        return Err(file_error("多文件上传不能指定统一名称，请留空名称"));
    }
    let mut pending = Vec::new();
    for root in roots {
        let remote_name = name
            .clone()
            .or_else(|| root.file_name().and_then(|v| v.to_str()).map(str::to_owned))
            .ok_or_else(|| file_error("文件名编码不受支持"))?;
        pending.push((root, child_path(directory, &remote_name)?, 0usize));
    }
    pending.reverse();
    let mut items = Vec::new();
    while let Some((local, remote, depth)) = pending.pop() {
        if depth > 64 || items.len() + pending.len() >= 10000 {
            return Err(file_error("上传目录超过 64 层或 10000 项限制"));
        }
        let attrs = tokio::fs::symlink_metadata(&local)
            .await
            .map_err(|_| file_error("无法读取上传源"))?;
        if attrs.file_type().is_symlink() || (!attrs.is_file() && !attrs.is_dir()) {
            return Err(file_error("不上传本地符号链接或特殊文件"));
        }
        let is_directory = attrs.is_dir();
        if is_directory {
            let mut children = tokio::fs::read_dir(&local)
                .await
                .map_err(|_| file_error("无法读取上传目录"))?;
            while let Some(child) = children
                .next_entry()
                .await
                .map_err(|_| file_error("读取上传目录失败"))?
            {
                if items.len() + pending.len() >= 10000 {
                    return Err(file_error("上传批次超过 10000 项限制"));
                }
                let child_name = child
                    .file_name()
                    .into_string()
                    .map_err(|_| file_error("文件名编码不受支持"))?;
                pending.push((child.path(), child_path(&remote, &child_name)?, depth + 1));
            }
        }
        items.push(UploadItem {
            local,
            remote,
            directory: is_directory,
        });
    }
    Ok(items)
}

async fn upload_paths(
    window: &WebviewWindow,
    state: &AppState,
    owner_id: &str,
    session_id: &str,
    browser: &SftpBrowser,
    paths: Vec<PathBuf>,
    directory: &str,
    name: Option<String>,
    cancel: &terminal_core::CancellationToken,
    on_progress: &Channel<TransferProgress>,
) -> Result<()> {
    if on_progress
        .send(TransferProgress {
            transferred: "0".into(),
            total: None,
            current_name: Some("正在扫描上传文件…".into()),
            completed_files: Some(0),
            total_files: None,
        })
        .is_err()
    {
        cancel.cancel();
    }
    // 先完成有界扫描，避免发现不支持条目后才修改远端；文件正文不进入计划。
    let items = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(SessionError::cancelled()),
        result = upload_items(paths, directory, name) => result?,
    };
    let total = items.iter().filter(|item| !item.directory).count();
    let mut completed = 0;
    let result: Result<()> = async {
        for item in items {
            if cancel.is_cancelled() { return Err(SessionError::cancelled()); }
            connected(state, owner_id, session_id)?;
            if item.directory {
                // 合并已有目录，不删除或替换任何目录；树内链接不是递归上传目标。
                if browser.upload_metadata(&item.remote).await?.is_some_and(|attrs| attrs.file_type().is_symlink()) {
                    return Err(file_error("上传树中的同名目标是链接，不自动合并，请直接打开目标目录后上传"));
                }
                browser.merge_directory(&item.remote).await?;
                continue;
            }
            let existing = browser.upload_metadata(&item.remote).await?;
            let overwrite = if let Some(attrs) = existing {
                if !attrs.file_type().is_file() {
                    return Err(file_error("同名目标是目录、链接或特殊文件，不予覆盖"));
                }
                let (tx, rx) = tokio::sync::oneshot::channel();
                window.dialog().message(format!("远端文件已存在：\n{}\n\n覆盖会直接截断并写入文件；中断后可能保留不完整内容。是否覆盖？", item.remote))
                    .title("确认覆盖文件").kind(MessageDialogKind::Warning)
                    .buttons(MessageDialogButtons::YesNoCancelCustom("保留并停止".into(), "覆盖".into(), "取消".into()))
                    .parent(window).show_with_result(move |answer| {
                        let _ = tx.send(matches!(answer, MessageDialogResult::Custom(label) if label == "覆盖"));
                    });
                let confirmed = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return Err(SessionError::cancelled()),
                    answer = rx => answer.map_err(|_| file_error("覆盖确认已关闭"))?,
                };
                if !confirmed {
                    let mut error = SessionError::cancelled();
                    error.message = "用户取消覆盖，批次已停止".into();
                    return Err(error);
                }
                connected(state, owner_id, session_id)?;
                true
            } else { false };
            if cancel.is_cancelled() { return Err(SessionError::cancelled()); }
            let attrs = tokio::fs::symlink_metadata(&item.local).await.map_err(|_| file_error("无法读取上传源"))?;
            if !attrs.is_file() || attrs.file_type().is_symlink() { return Err(file_error("上传源已变化，不再是普通文件")); }
            let mut source = tokio::fs::File::open(&item.local).await.map_err(|_| file_error("无法打开上传源"))?;
            let opened = source.metadata().await.map_err(|_| file_error("无法核对上传源属性"))?;
            if !opened.is_file() || opened.len() != attrs.len() {
                return Err(file_error("上传源在打开前发生变化，请重试"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if opened.dev() != attrs.dev() || opened.ino() != attrs.ino() {
                    return Err(file_error("上传源在打开前被替换，请重试"));
                }
            }
            let mut target = browser.writer(&item.remote, overwrite).await?;
            copy_file(&mut source, &mut target, Some(attrs.len()), cancel, |mut progress| {
                progress.current_name = Some(item.remote.clone());
                progress.completed_files = Some(completed);
                progress.total_files = Some(total);
                if on_progress.send(progress).is_err() { cancel.cancel(); }
            }).await?;
            completed += 1;
            if on_progress.send(TransferProgress {
                transferred: attrs.len().to_string(), total: Some(attrs.len().to_string()),
                current_name: Some(item.remote), completed_files: Some(completed), total_files: Some(total),
            }).is_err() { cancel.cancel(); }
        }
        Ok(())
    }.await;
    result.map_err(|mut error| {
        error.message = format!(
            "{}；已完成 {completed}/{total} 个文件。直接写入的目标可能保留部分内容，目录均未删除",
            error.message
        );
        error
    })
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
    let cancel = terminal_core::CancellationToken::new();
    *browser.transfer.lock().unwrap() = Some((transfer_id, cancel.clone()));
    let _transfer = TransferGuard(browser.clone());
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
    let cancel = terminal_core::CancellationToken::new();
    *browser.transfer.lock().unwrap() = Some((transfer_id, cancel.clone()));
    let _transfer = TransferGuard(browser.clone());
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

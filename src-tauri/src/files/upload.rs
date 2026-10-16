use super::connected;
use crate::state::AppState;
use std::path::PathBuf;
use tauri::{WebviewWindow, ipc::Channel};
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};
use terminal_core::{Result, SessionError};
use terminal_files::{TransferProgress, child_path, copy_file, file_error};
use terminal_sftp::SftpBrowser;

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

pub(crate) async fn upload_paths(
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

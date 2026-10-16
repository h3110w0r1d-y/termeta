use std::path::Path;
use terminal_core::{Result, SessionError};
use terminal_files::{FileProvider, TransferProgress, child_path, copy_file, file_error};
use terminal_sftp::SftpBrowser;

/// 原生拖出与保存对话框共用；调用者负责会话所有权、操作锁和传输许可。
pub(crate) async fn download_to_path(
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
pub(crate) async fn download_directory_to_path(
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

//! 跨 SFTP 端点有界中转；先扫描元数据，再合并目录及直接写入文件。
use crate::SftpBrowser;
use std::{collections::HashSet, future::Future};
use terminal_core::{CancellationToken, Result, SessionError};
use terminal_files::{TransferProgress, child_path, copy_file, file_error, validate_path};

struct Item {
    source: String,
    target: String,
    directory: bool,
}

pub async fn copy_between<F, Fut>(
    source: &SftpBrowser,
    target: &SftpBrowser,
    source_path: &str,
    target_path: &str,
    names: Vec<String>,
    cancel: &CancellationToken,
    confirm: F,
    progress: impl Fn(TransferProgress),
) -> Result<()>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<bool>>,
{
    validate_path(source_path)?;
    validate_path(target_path)?;
    // 只接受浏览器已返回的绝对逻辑路径，不能用相对路径绕过自复制检查。
    for path in [source_path, target_path] {
        if !path.starts_with('/') || path.split('/').any(|p| matches!(p, "." | "..")) {
            return Err(file_error("请刷新目录后再对拷"));
        }
    }
    let mut seen = HashSet::new();
    let mut pending = Vec::new();
    for name in names.into_iter().rev() {
        if !seen.insert(name.clone()) {
            return Err(file_error("对拷列表含重复名称"));
        }
        pending.push((
            child_path(source_path, &name)?,
            child_path(target_path, &name)?,
            0,
        ));
    }
    let mut items = Vec::new();
    while let Some((from, to, depth)) = pending.pop() {
        if cancel.is_cancelled() {
            return Err(SessionError::cancelled());
        }
        if depth > 64 || items.len() + pending.len() >= 10000 {
            return Err(file_error("对拷超过64层或10000项限制"));
        }
        let attrs = source.metadata(&from).await?;
        let directory = attrs.file_type().is_dir();
        if !directory && !attrs.file_type().is_file() {
            return Err(file_error("对拷不跟随软链接，也不复制特殊文件"));
        }
        if directory {
            let entries = source
                .scan_directory(&from, 10000 - items.len() - pending.len(), cancel)
                .await?;
            for entry in entries.into_iter().rev() {
                pending.push((
                    child_path(&from, &entry.name)?,
                    child_path(&to, &entry.name)?,
                    depth + 1,
                ));
            }
        }
        items.push(Item {
            source: from,
            target: to,
            directory,
        });
    }
    let total = items.iter().filter(|item| !item.directory).count();
    let mut completed = 0;
    let result: Result<()> = async {
        for item in items {
            if cancel.is_cancelled() {
                return Err(SessionError::cancelled());
            }
            if item.directory {
                target.merge_directory(&item.target).await?;
                continue;
            }
            let overwrite = target.confirm_upload(&item.target, &confirm).await?;
            if cancel.is_cancelled() {
                return Err(SessionError::cancelled());
            }
            let attrs = source.metadata(&item.source).await?;
            let mut reader = source.reader(&item.source).await?;
            let mut writer = target.writer(&item.target, overwrite).await?;
            copy_file(&mut reader, &mut writer, attrs.size, cancel, |mut value| {
                value.current_name = Some(item.target.clone());
                value.completed_files = Some(completed);
                value.total_files = Some(total);
                progress(value);
            })
            .await?;
            completed += 1;
            progress(TransferProgress {
                transferred: attrs.size.unwrap_or(0).to_string(),
                total: attrs.size.map(|size| size.to_string()),
                current_name: Some(item.target),
                completed_files: Some(completed),
                total_files: Some(total),
            });
        }
        Ok(())
    }
    .await;
    result.map_err(|mut error| {
        error.message = format!(
            "{}；已完成 {completed}/{total} 个文件，目标可能保留部分内容，目录均未删除",
            error.message
        );
        error
    })
}

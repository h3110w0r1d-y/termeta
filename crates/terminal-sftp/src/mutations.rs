//! 远端元数据操作；所有递归扫描在写入前完成，并保持有界内存与取消边界。
use crate::{SftpBrowser, eof, protocol_error};
use russh_sftp::protocol::FileAttributes;
use std::collections::HashSet;
use terminal_core::{CancellationToken, Result, SessionError};
use terminal_files::{child_path, file_error, validate_path};

const MAX_ROOTS: usize = 100;
const MAX_ENTRIES: usize = 10_000;
const MAX_DEPTH: usize = 64;

struct Removal {
    path: String,
    directory: bool,
}

fn roots(directory: &str, names: &[String]) -> Result<Vec<String>> {
    validate_path(directory)?;
    if !directory.starts_with('/') || names.is_empty() || names.len() > MAX_ROOTS {
        return Err(file_error("请选择 1 至 100 项，并刷新到有效的绝对目录"));
    }
    let mut unique = HashSet::new();
    names
        .iter()
        .map(|name| {
            if !unique.insert(name) {
                return Err(file_error("选择中存在重复文件名"));
            }
            child_path(directory, name)
        })
        .collect()
}

impl SftpBrowser {
    /// 删除链接本身，绝不有意递归链接目标；SFTPv3 不提供无竞态的路径操作。
    pub async fn remove_entries(
        &self,
        directory: &str,
        names: &[String],
        recursive: bool,
        cancel: &CancellationToken,
    ) -> Result<()> {
        let mut pending: Vec<_> = roots(directory, names)?
            .into_iter()
            .map(|path| (path, 0usize))
            .collect();
        let mut removals = Vec::new();
        // 先完成预扫描，超限、非法名称或无权读取时不删除任何条目。
        while let Some((path, depth)) = pending.pop() {
            if depth > MAX_DEPTH || removals.len() >= MAX_ENTRIES {
                return Err(file_error("删除范围超过 64 层或 10000 项限制"));
            }
            let attrs = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(SessionError::cancelled()),
                attrs = self.metadata(&path) => attrs?,
            };
            let directory = attrs.file_type().is_dir();
            if directory && recursive {
                let remaining = MAX_ENTRIES.saturating_sub(removals.len() + pending.len() + 1);
                let children = self.removal_children(&path, remaining, cancel).await?;
                pending.extend(children.into_iter().map(|child| (child, depth + 1)));
            }
            removals.push(Removal { path, directory });
        }
        // 逆先序确保子项先于目录删除；执行失败或取消时已经删除的条目无法恢复。
        for entry in removals.into_iter().rev() {
            let remove = async {
                let attrs = self.metadata(&entry.path).await?;
                if attrs.file_type().is_dir() != entry.directory {
                    return Err(file_error("删除对象类型已变化，请刷新后重试"));
                }
                if entry.directory {
                    self.raw.rmdir(&entry.path).await.map_err(protocol_error)?;
                } else {
                    self.raw.remove(&entry.path).await.map_err(protocol_error)?;
                }
                Ok::<(), SessionError>(())
            };
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(SessionError::cancelled()),
                result = remove => result?,
            }
        }
        Ok(())
    }

    async fn removal_children(
        &self,
        path: &str,
        maximum: usize,
        cancel: &CancellationToken,
    ) -> Result<Vec<String>> {
        if cancel.is_cancelled() {
            return Err(SessionError::cancelled());
        }
        // 等待打开返回后再处理取消，才能拿到并关闭服务器已分配的句柄；请求受 15 秒超时限制。
        let handle = self.raw.opendir(path).await.map_err(protocol_error)?.handle;
        let read = async {
            let mut children = Vec::new();
            let mut unique = HashSet::new();
            for _ in 0..20_000 {
                match self.raw.readdir(&handle).await {
                    Ok(batch) => {
                        for entry in batch.files {
                            if matches!(entry.filename.as_str(), "." | "..") {
                                continue;
                            }
                            if children.len() >= maximum {
                                return Err(file_error("删除范围超过 10000 项限制"));
                            }
                            if !unique.insert(entry.filename.clone()) {
                                return Err(file_error("目录返回重复条目，请刷新后重试"));
                            }
                            children.push(child_path(path, &entry.filename)?);
                        }
                    }
                    Err(error) if eof(&error) => return Ok(children),
                    Err(error) => return Err(protocol_error(error)),
                }
            }
            Err(file_error("目录返回过多批次，删除已停止"))
        };
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(SessionError::cancelled()),
            result = read => result,
        };
        let closed = self.raw.close(&handle).await.map_err(protocol_error);
        let children = result?;
        closed?;
        Ok(children)
    }

    /// 只修改所选普通文件/目录本身，保留所有者、时间和大小；不递归并拒绝已知链接。
    /// SFTPv3 的 SETSTAT 无 NOFOLLOW，LSTAT 复核不能消除并发替换竞态。
    pub async fn set_entry_permissions(
        &self,
        directory: &str,
        names: &[String],
        permissions: u32,
        cancel: &CancellationToken,
    ) -> Result<()> {
        if permissions > 0o7777 {
            return Err(file_error("权限必须为 0000 至 7777 的八进制值"));
        }
        let paths = roots(directory, names)?;
        // 先拒绝整批中的链接和特殊文件，避免已知的不支持对象导致部分写入。
        for path in &paths {
            let attrs = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(SessionError::cancelled()),
                attrs = self.metadata(path) => attrs?,
            };
            if !attrs.file_type().is_file() && !attrs.file_type().is_dir() {
                return Err(file_error("只允许修改普通文件或目录权限，不修改链接目标"));
            }
        }
        for path in paths {
            let update = async {
                let attrs = self.metadata(&path).await?;
                if !attrs.file_type().is_file() && !attrs.file_type().is_dir() {
                    return Err(file_error("权限编辑对象已变化，请刷新后重试"));
                }
                self.raw
                    .setstat(
                        path,
                        FileAttributes {
                            permissions: Some(permissions),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(protocol_error)?;
                Ok::<(), SessionError>(())
            };
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(SessionError::cancelled()),
                result = update => result?,
            }
        }
        Ok(())
    }
}

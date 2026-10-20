use crate::{RemoteFile, eof, protocol_error};
use async_trait::async_trait;
use russh_sftp::{
    client::{Config, RawSftpSession, error::Error},
    protocol::{FileAttributes, OpenFlags, StatusCode},
};
use std::{collections::VecDeque, sync::Arc};
use terminal_core::Result;
use terminal_files::*;
use tokio::sync::Mutex;

struct Listing {
    path: String,
    handle: Option<String>,
    count: usize,
    pending: VecDeque<FileEntry>,
}
pub struct SftpBrowser {
    raw: Arc<RawSftpSession>,
    identity: terminal_ssh::EndpointIdentity,
    lifetime: terminal_core::CancellationToken,
    listing: Mutex<Option<Listing>>,
    // 壳使用此门限序列化每个浏览器的操作，不阻塞终端 actor。
    pub operation: Mutex<()>,
    pub transfer: std::sync::Mutex<Option<(String, terminal_core::CancellationToken)>>,
}
impl Drop for SftpBrowser {
    fn drop(&mut self) {
        if let Some((_, cancel)) = self.transfer.lock().unwrap().take() {
            cancel.cancel();
        }
        let _ = self.raw.close_session();
    }
}
impl SftpBrowser {
    pub async fn connect(
        channels: &terminal_ssh::SshChannels,
        owner: &str,
        session_id: &str,
        lifetime: terminal_core::CancellationToken,
    ) -> Result<Self> {
        let identity = channels.endpoint_identity(owner, session_id)?;
        let stream = channels.open_sftp(owner, session_id).await?;
        let raw = RawSftpSession::new_with_config(
            stream,
            Config {
                max_packet_len: 256 * 1024,
                max_concurrent_reads: 1,
                max_concurrent_writes: 1,
                max_write_packet_len: 64 * 1024,
                request_timeout_secs: 15,
            },
        );
        raw.init().await.map_err(protocol_error)?;
        Ok(Self {
            raw: Arc::new(raw),
            identity,
            lifetime,
            listing: Mutex::new(None),
            operation: Mutex::new(()),
            transfer: std::sync::Mutex::new(None),
        })
    }
    pub fn identity(&self) -> &terminal_ssh::EndpointIdentity {
        &self.identity
    }
    /// 迟到的传输登记也继承已取消状态，避免断线清理与登记交错后遗留原生等待。
    pub fn transfer_cancellation(&self) -> terminal_core::CancellationToken {
        self.lifetime.child_token()
    }
    pub fn register_transfer(&self, id: &str, cancel: terminal_core::CancellationToken) {
        let mut transfer = self.transfer.lock().unwrap();
        if self.lifetime.is_cancelled() {
            cancel.cancel();
        }
        *transfer = Some((id.into(), cancel));
    }
    pub fn cancel(&self) {
        self.lifetime.cancel();
        if let Some((_, cancel)) = self.transfer.lock().unwrap().as_ref() {
            cancel.cancel();
        }
        let _ = self.raw.close_session();
    }
    pub fn cancel_transfer(&self, transfer_id: &str) {
        if let Some((id, cancel)) = self.transfer.lock().unwrap().as_ref() {
            if id == transfer_id {
                cancel.cancel();
            }
        }
    }
    /// 仅用于同端点自复制检查；浏览路径仍保持软链接逻辑路径。
    pub async fn canonical_path(&self, path: &str) -> Result<String> {
        validate_path(path)?;
        self.raw
            .realpath(path)
            .await
            .map_err(protocol_error)?
            .files
            .into_iter()
            .next()
            .map(|entry| entry.filename)
            .ok_or_else(|| file_error("服务器未返回实际路径"))
    }
    pub async fn metadata(&self, path: &str) -> Result<FileAttributes> {
        validate_path(path)?;
        self.raw
            .lstat(path)
            .await
            .map(|attrs| attrs.attrs)
            .map_err(protocol_error)
    }
    pub async fn reader(&self, path: &str) -> Result<RemoteFile> {
        let attrs = self.metadata(path).await?;
        if !attrs.file_type().is_file() {
            return Err(file_error("仅支持传输普通文件，符号链接和特殊文件不可下载"));
        }
        let handle = self
            .raw
            .open(path, OpenFlags::READ, FileAttributes::empty())
            .await
            .map_err(protocol_error)?
            .handle;
        let actual = self.raw.fstat(&handle).await.map_err(protocol_error);
        if !actual
            .as_ref()
            .is_ok_and(|attrs| attrs.attrs.file_type().is_file())
        {
            let _ = self.raw.close(&handle).await;
            return Err(file_error("打开后的源文件不是普通文件"));
        }
        Ok(RemoteFile {
            raw: self.raw.clone(),
            handle: Some(handle),
            offset: 0,
        })
    }
    /// 目录传输使用独立扫描句柄，不改变浏览器分页游标；取消后仍关闭句柄。
    pub async fn scan_directory(
        &self,
        path: &str,
        maximum: usize,
        cancel: &terminal_core::CancellationToken,
    ) -> Result<Vec<FileEntry>> {
        validate_path(path)?;
        if !self.metadata(path).await?.file_type().is_dir() {
            return Err(file_error("目录传输源已变化，拒绝跟随软连接或特殊文件"));
        }
        if cancel.is_cancelled() {
            return Err(terminal_core::SessionError::cancelled());
        }
        let handle = self.raw.opendir(path).await.map_err(protocol_error)?.handle;
        let read = async {
            let mut entries = Vec::new();
            let mut requests = 0usize;
            loop {
                requests += 1;
                if requests > 20000 {
                    return Err(file_error("目录返回过多批次，下载已停止"));
                }
                match self.raw.readdir(&handle).await {
                    Ok(batch) => {
                        for file in batch.files {
                            if matches!(file.filename.as_str(), "." | "..") {
                                continue;
                            }
                            validate_name(&file.filename)?;
                            if entries.len() >= maximum {
                                return Err(file_error("目录传输超过 10000 项限制"));
                            }
                            let kind = match file.attrs.file_type() {
                                russh_sftp::protocol::FileType::Dir => "directory",
                                russh_sftp::protocol::FileType::File => "file",
                                _ => {
                                    return Err(file_error(
                                        "目录含软连接或特殊文件，当前不递归传输这些条目",
                                    ));
                                }
                            };
                            entries.push(FileEntry {
                                name: file.filename,
                                kind: kind.into(),
                                link_target: None,
                                target_kind: None,
                                size: file.attrs.size.map(|n| n.to_string()),
                                modified: file.attrs.mtime,
                                permissions: file.attrs.permissions,
                            });
                        }
                    }
                    Err(error) if eof(&error) => return Ok(entries),
                    Err(error) => return Err(protocol_error(error)),
                }
            }
        };
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(terminal_core::SessionError::cancelled()),
            result = read => result,
        };
        let closed = self.raw.close(&handle).await.map_err(protocol_error);
        let entries = result?;
        closed?;
        Ok(entries)
    }
    /// 缺失目标只接受协议明确返回的 NoSuchFile，权限和网络错误不可当作不存在。
    pub async fn upload_metadata(&self, path: &str) -> Result<Option<FileAttributes>> {
        validate_path(path)?;
        match self.raw.lstat(path).await {
            Ok(attrs) => Ok(Some(attrs.attrs)),
            Err(Error::Status(status)) if status.status_code == StatusCode::NoSuchFile => Ok(None),
            Err(error) => Err(protocol_error(error)),
        }
    }
    /// 目录上传只合并，不删除同名目标；壳另行限制树内部链接，显式目标路径仍由服务器跟随。
    pub async fn merge_directory(&self, path: &str) -> Result<()> {
        if let Some(attrs) = self.upload_metadata(path).await? {
            if attrs.file_type().is_dir() {
                return Ok(());
            }
            if attrs.file_type().is_symlink()
                && self
                    .raw
                    .stat(path)
                    .await
                    .map_err(protocol_error)?
                    .attrs
                    .file_type()
                    .is_dir()
            {
                return Ok(());
            }
            return Err(file_error("远端同名目标不是目录，不能合并或替换"));
        }
        self.raw
            .mkdir(
                path,
                FileAttributes {
                    permissions: Some(0o700),
                    ..Default::default()
                },
            )
            .await
            .map(|_| ())
            .map_err(protocol_error)
    }
    pub async fn writer(&self, path: &str, overwrite: bool) -> Result<RemoteFile> {
        validate_path(path)?;
        let existing = self.upload_metadata(path).await?;
        if let Some(attrs) = &existing {
            if !attrs.file_type().is_file() {
                return Err(file_error(
                    "同名目标不是普通文件，不覆盖目录、链接或特殊文件",
                ));
            }
            if !overwrite {
                return Err(file_error("远端同名文件已存在，尚未确认覆盖"));
            }
        } else if overwrite {
            return Err(file_error("覆盖确认后目标已变化，请重试"));
        }
        let flags = if overwrite {
            OpenFlags::WRITE
        } else {
            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::EXCLUDE
        };
        let handle = self
            .raw
            .open(
                path,
                flags,
                FileAttributes {
                    permissions: Some(0o600),
                    ..Default::default()
                },
            )
            .await
            .map_err(protocol_error)?
            .handle;
        // 覆盖前核对已打开句柄类型，再截断；不通过删除目录或链接实现覆盖。
        let actual = self.raw.fstat(&handle).await.map_err(protocol_error);
        if !actual
            .as_ref()
            .is_ok_and(|attrs| attrs.attrs.file_type().is_file())
        {
            let _ = self.raw.close(&handle).await;
            return Err(file_error("打开后的上传目标不是普通文件"));
        }
        if overwrite {
            if let Err(error) = self
                .raw
                .fsetstat(
                    &handle,
                    FileAttributes {
                        size: Some(0),
                        ..Default::default()
                    },
                )
                .await
            {
                let _ = self.raw.close(&handle).await;
                return Err(protocol_error(error));
            }
        }
        Ok(RemoteFile {
            raw: self.raw.clone(),
            handle: Some(handle),
            offset: 0,
        })
    }
}
#[async_trait]
impl FileProvider for SftpBrowser {
    async fn list(&self, path: &str, more: bool) -> Result<DirectoryPage> {
        validate_path(path)?;
        let mut listing = self.listing.lock().await;
        if !more {
            if let Some(old) = listing.take() {
                if let Some(handle) = old.handle {
                    let _ = self.raw.close(handle).await;
                }
            }
            // 只解析会话初始目录；用户路径按逻辑层级归一，避免REALPATH展开软连接。
            let absolute = if path.starts_with('/') {
                path.to_string()
            } else {
                let home = self
                    .raw
                    .realpath(".")
                    .await
                    .map_err(protocol_error)?
                    .files
                    .into_iter()
                    .next()
                    .ok_or_else(|| file_error("服务器未返回初始目录"))?
                    .filename;
                validate_path(&home)?;
                if !home.starts_with('/') {
                    return Err(file_error("服务器初始目录不是绝对路径"));
                }
                format!("{}/{path}", home.trim_end_matches('/'))
            };
            let mut parts = Vec::new();
            for part in absolute.split('/') {
                match part {
                    "" | "." => {}
                    ".." => {
                        parts.pop();
                    }
                    _ => parts.push(part),
                }
            }
            let logical = format!("/{}", parts.join("/"));
            validate_path(&logical)?;
            let handle = self
                .raw
                .opendir(&logical)
                .await
                .map_err(protocol_error)?
                .handle;
            *listing = Some(Listing {
                path: logical,
                handle: Some(handle),
                count: 0,
                pending: VecDeque::new(),
            });
        }
        let current = listing
            .as_mut()
            .ok_or_else(|| file_error("目录游标已过期，请刷新"))?;
        if more && path != current.path {
            return Err(file_error("目录游标与路径不匹配"));
        }
        if current.count >= 10_000 {
            return Err(file_error(
                "目录超过当前视图 10000 项上限，请直接输入子目录路径",
            ));
        }
        let mut entries = Vec::with_capacity(100);
        let mut requests = 0;
        // 协议包大小不等于页面大小；只缓存当前包余项，下一次继续消费，不丢弃条目。
        while entries.len() < 100 && current.count + entries.len() < 10_000 {
            if let Some(entry) = current.pending.pop_front() {
                entries.push(entry);
                continue;
            }
            let Some(handle) = current.handle.as_ref() else {
                break;
            };
            requests += 1;
            if requests > 64 {
                return Err(file_error("目录返回过多空批次，请刷新重试"));
            }
            match self.raw.readdir(handle).await {
                Ok(name) => {
                    for file in name.files {
                        if file.filename == "." || file.filename == ".." {
                            continue;
                        }
                        validate_name(&file.filename)?;
                        let kind = match file.attrs.file_type() {
                            russh_sftp::protocol::FileType::Dir => "directory",
                            russh_sftp::protocol::FileType::File => "file",
                            russh_sftp::protocol::FileType::Symlink => "symlink",
                            _ => "other",
                        };
                        current.pending.push_back(FileEntry {
                            name: file.filename,
                            kind: kind.into(),
                            link_target: None,
                            target_kind: None,
                            size: file.attrs.size.map(|n| n.to_string()),
                            modified: file.attrs.mtime,
                            permissions: file.attrs.permissions,
                        });
                    }
                }
                Err(error) => {
                    if let Some(handle) = current.handle.take() {
                        let _ = self.raw.close(handle).await;
                    }
                    if !eof(&error) {
                        return Err(protocol_error(error));
                    }
                }
            }
        }
        // 只解析本页链接，每批最多8个；读取目标失败不阻断普通目录浏览。
        let links: Vec<_> = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.kind == "symlink")
            .map(|(index, entry)| Ok((index, child_path(&current.path, &entry.name)?)))
            .collect::<Result<_>>()?;
        for batch in links.chunks(8) {
            let mut tasks = tokio::task::JoinSet::new();
            for (index, path) in batch {
                let raw = self.raw.clone();
                let index = *index;
                let path = path.clone();
                tasks.spawn(async move {
                    let (target, attrs) = tokio::join!(raw.readlink(&path), raw.stat(&path));
                    let target = target
                        .ok()
                        .and_then(|name| name.files.into_iter().next().map(|file| file.filename));
                    let kind = attrs.ok().map(|attrs| {
                        match attrs.attrs.file_type() {
                            russh_sftp::protocol::FileType::Dir => "directory",
                            russh_sftp::protocol::FileType::File => "file",
                            _ => "other",
                        }
                        .to_string()
                    });
                    (index, target, kind)
                });
            }
            while let Some(result) = tasks.join_next().await {
                if let Ok((index, target, kind)) = result {
                    entries[index].link_target = target;
                    entries[index].target_kind = kind;
                }
            }
        }
        current.count += entries.len();
        Ok(DirectoryPage {
            path: current.path.clone(),
            entries,
            has_more: !current.pending.is_empty() || current.handle.is_some(),
        })
    }
}

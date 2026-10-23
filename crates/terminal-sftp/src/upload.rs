//! 上传与对拷共用的远端写入边界，不持有本地授权或原生确认界面。
use crate::{RemoteFile, SftpBrowser, protocol_error};
use russh_sftp::{
    client::error::Error,
    protocol::{FileAttributes, OpenFlags, StatusCode},
};
use std::future::Future;
use terminal_core::{Result, SessionError};
use terminal_files::{file_error, validate_path};

impl SftpBrowser {
    /// 缺失目标只接受协议明确返回的 NoSuchFile，权限和网络错误不可当作不存在。
    async fn upload_metadata(&self, path: &str) -> Result<Option<FileAttributes>> {
        validate_path(path)?;
        match self.raw.lstat(path).await {
            Ok(attrs) => Ok(Some(attrs.attrs)),
            Err(Error::Status(status)) if status.status_code == StatusCode::NoSuchFile => Ok(None),
            Err(error) => Err(protocol_error(error)),
        }
    }
    /// 上传与对拷只合并真实目录；已进入目录的逻辑路径前缀仍由服务端解析。
    pub async fn merge_directory(&self, path: &str) -> Result<()> {
        if let Some(attrs) = self.upload_metadata(path).await? {
            if attrs.file_type().is_dir() {
                return Ok(());
            }
            if attrs.file_type().is_symlink() {
                return Err(file_error("同名目录是软链接，不自动合并"));
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
    /// 仅确认普通文件的覆盖意图；源打开后仍须由 writer 复核目标并安全截断。
    /// 确认回调由壳适配原生对话框和取消，不在此处依赖窗口或打开目标句柄。
    pub async fn confirm_upload<F, Fut>(&self, path: &str, confirm: F) -> Result<bool>
    where
        F: FnOnce(String) -> Fut,
        Fut: Future<Output = Result<bool>>,
    {
        match self.upload_metadata(path).await? {
            Some(attrs) if attrs.file_type().is_file() => {
                if !confirm(path.to_owned()).await? {
                    return Err(SessionError::cancelled());
                }
                Ok(true)
            }
            Some(_) => Err(file_error("同名目标是目录、链接或特殊文件，不予覆盖")),
            None => Ok(false),
        }
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

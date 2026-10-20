use crate::{eof, protocol_error};
use async_trait::async_trait;
use russh_sftp::client::RawSftpSession;
use std::sync::Arc;
use terminal_core::Result;
use terminal_files::{FileReader, FileWriter, file_error};

impl Drop for RemoteFile {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let raw = self.raw.clone();
            // 提前返回仍释放远端句柄，不持有浏览器或全局注册表锁等待。
            tokio::spawn(async move {
                let _ = raw.close(handle).await;
            });
        }
    }
}
pub struct RemoteFile {
    pub(super) raw: Arc<RawSftpSession>,
    pub(super) handle: Option<String>,
    pub(super) offset: u64,
}
impl RemoteFile {
    /// SFTP 使用显式字节偏移；原生虚拟文件流可独立定位，不读取或缓存跳过内容。
    pub fn seek_to(&mut self, offset: u64) {
        self.offset = offset;
    }

    /// 以打开后的句柄核对长度，避免原生拖出使用打开前的路径属性声明流大小。
    pub async fn size(&self) -> Result<Option<u64>> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| file_error("文件已关闭"))?;
        self.raw
            .fstat(handle)
            .await
            .map(|value| value.attrs.size)
            .map_err(protocol_error)
    }

    async fn finish(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            self.raw.close(handle).await.map_err(protocol_error)?;
        }
        Ok(())
    }
}
#[async_trait]
impl FileReader for RemoteFile {
    async fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| file_error("文件已关闭"))?;
        match self
            .raw
            .read(handle, self.offset, buffer.len() as u32)
            .await
        {
            Ok(data) => {
                if data.data.len() > buffer.len() {
                    return Err(file_error("服务器返回的数据超过请求大小"));
                }
                let count = data.data.len();
                buffer[..count].copy_from_slice(&data.data);
                self.offset += count as u64;
                Ok(count)
            }
            Err(error) if eof(&error) => Ok(0),
            Err(error) => Err(protocol_error(error)),
        }
    }
    async fn close(&mut self) -> Result<()> {
        self.finish().await
    }
}
#[async_trait]
impl FileWriter for RemoteFile {
    async fn write_chunk(&mut self, bytes: &[u8]) -> Result<()> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| file_error("文件已关闭"))?;
        self.raw
            .write(handle, self.offset, bytes.to_vec())
            .await
            .map_err(protocol_error)?;
        self.offset += bytes.len() as u64;
        Ok(())
    }
    async fn close(&mut self) -> Result<()> {
        self.finish().await
    }
}

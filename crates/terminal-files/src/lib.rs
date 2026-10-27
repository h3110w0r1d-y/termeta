//! 文件元数据、受界流式传输与协议无关端口；不接触终端输出或认证秘密。
use async_trait::async_trait;
use serde::Serialize;
use terminal_core::{Result, SessionError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub kind: String,
    /// 软连接的原始目标与跟随后的类型；服务器拒绝或目标失效时可为空。
    pub link_target: Option<String>,
    pub target_kind: Option<String>,
    pub size: Option<String>,
    pub modified: Option<u32>,
    pub permissions: Option<u32>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryPage {
    pub path: String,
    /// 每页最多100个远端条目，不包含前端生成的上级目录导航行。
    pub entries: Vec<FileEntry>,
    pub has_more: bool,
}
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub transferred: String,
    pub total: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_files: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_files: Option<usize>,
}
pub fn file_error(message: &str) -> SessionError {
    SessionError::new("FILE_OPERATION_FAILED", "files", message)
}
pub fn validate_path(path: &str) -> Result<()> {
    if path.is_empty() || path.len() > 4096 || path.contains('\0') {
        return Err(file_error("路径为空、过长或包含 NUL"));
    }
    Ok(())
}
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 255
        || matches!(name, "." | "..")
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(file_error("请输入有效的单个文件名（不含路径分隔符）"));
    }
    Ok(())
}
pub fn child_path(directory: &str, name: &str) -> Result<String> {
    validate_name(name)?;
    let path = format!("{}/{name}", directory.trim_end_matches('/'));
    validate_path(&path)?;
    Ok(path)
}
#[async_trait]
pub trait FileProvider: Send + Sync {
    async fn list(&self, path: &str, more: bool) -> Result<DirectoryPage>;
}
#[async_trait]
pub trait FileReader: Send {
    async fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize>;
    async fn close(&mut self) -> Result<()>;
}
#[async_trait]
pub trait FileWriter: Send {
    async fn write_chunk(&mut self, bytes: &[u8]) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
}
#[async_trait]
impl FileReader for tokio::fs::File {
    async fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.read(buffer)
            .await
            .map_err(|_| file_error("读取本地文件失败"))
    }
    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
#[async_trait]
impl FileWriter for tokio::fs::File {
    async fn write_chunk(&mut self, bytes: &[u8]) -> Result<()> {
        self.write_all(bytes)
            .await
            .map_err(|_| file_error("写入本地文件失败"))
    }
    async fn close(&mut self) -> Result<()> {
        self.sync_all()
            .await
            .map_err(|_| file_error("保存本地文件失败"))
    }
}
/// 每任务仅一个 64 KiB 缓冲；取消/错误不重放，关闭确认前不宣称传输完成。
pub async fn copy_file(
    reader: &mut dyn FileReader,
    writer: &mut dyn FileWriter,
    total: Option<u64>,
    cancel: &terminal_core::CancellationToken,
    progress: impl Fn(TransferProgress),
) -> Result<u64> {
    progress(TransferProgress {
        transferred: "0".into(),
        total: total.map(|n| n.to_string()),
        current_name: None,
        completed_files: None,
        total_files: None,
    });
    let mut buffer = vec![0; 64 * 1024];
    let mut transferred = 0u64;
    let mut last = std::time::Instant::now();
    let result = async {
        loop {
            let count = reader.read_chunk(&mut buffer).await?;
            if count == 0 {
                break;
            }
            if total.is_some_and(|total| transferred.saturating_add(count as u64) > total) {
                return Err(file_error("源文件大小已变化，传输未完成"));
            }
            writer.write_chunk(&buffer[..count]).await?;
            transferred = transferred
                .checked_add(count as u64)
                .ok_or_else(|| file_error("文件大小超出范围"))?;
            if last.elapsed() >= std::time::Duration::from_millis(150) {
                progress(TransferProgress {
                    transferred: transferred.to_string(),
                    total: total.map(|n| n.to_string()),
                    current_name: None,
                    completed_files: None,
                    total_files: None,
                });
                last = std::time::Instant::now();
            }
        }
        if total.is_some_and(|total| total != transferred) {
            return Err(file_error("源文件大小已变化，传输未完成"));
        }
        Ok(transferred)
    };
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(SessionError::cancelled()),
        result = result => result,
    };
    // 即使用户取消，也尝试关闭两端；每个协议请求仍受 provider 超时约束。
    let read_closed = reader.close().await;
    let write_closed = writer.close().await;
    let count = result?;
    read_closed?;
    write_closed?;
    progress(TransferProgress {
        transferred: count.to_string(),
        total: total.map(|n| n.to_string()),
        current_name: None,
        completed_files: None,
        total_files: None,
    });
    Ok(count)
}

//! 独立 SFTP 子系统适配器；目录增量读取，文件字节不进入 WebView。
mod browser;
mod copy;
mod mutations;
mod remote_file;
pub use browser::SftpBrowser;
pub use copy::copy_between;
pub use remote_file::RemoteFile;

use russh_sftp::{client::error::Error, protocol::StatusCode};
use terminal_files::file_error;

fn protocol_error(error: Error) -> terminal_core::SessionError {
    // 只展示协议状态，不回显可能携带文件正文的解析诊断。
    let message = match error {
        Error::Status(status) => format!("SFTP 请求失败：{:?}", status.status_code),
        _ => "SFTP 请求失败或超时，请刷新后重试".into(),
    };
    file_error(&message)
}
fn eof(error: &Error) -> bool {
    matches!(error, Error::Status(status) if status.status_code == StatusCode::Eof)
}

//! Telnet TCP 适配器；明文登录由用户在终端完成，不访问 SSH 凭据。
mod protocol;

use async_trait::async_trait;
use std::time::Duration;
use terminal_core::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};

pub struct TelnetFactory;
#[async_trait]
impl SessionFactory for TelnetFactory {
    fn kind(&self) -> &'static str {
        "telnet"
    }
    fn capabilities(&self) -> SessionCapabilities {
        SessionCapabilities {
            resize: true,
            pty: false,
            binary_output: true,
            serial_control: false,
        }
    }
    fn validate(&self, config: &SessionConfig) -> Result<()> {
        match config {
            SessionConfig::Telnet(config) => config.validate(),
            _ => Err(SessionError::new(
                "UNSUPPORTED_KIND",
                "opening",
                "Telnet 工厂仅接受 Telnet 配置",
            )),
        }
    }
    async fn run(&self, config: SessionConfig, mut context: DriverContext) -> Result<SessionEnd> {
        self.validate(&config)?;
        let SessionConfig::Telnet(config) = config else {
            unreachable!()
        };
        context.events.state(SessionState::Connecting)?;
        // DNS 与 TCP 建连共用截止时间，关闭通过独立取消信号打断。
        let stream = tokio::select! {
            biased;
            _ = context.cancel.cancelled() => return Err(SessionError::cancelled()),
            result = tokio::time::timeout(Duration::from_secs(30), TcpStream::connect((config.address.as_str(), config.port))) => {
                result.map_err(|_| SessionError::new("CONNECT_TIMEOUT", "connecting", "Telnet 连接超时"))?
                    .map_err(|error| SessionError::new("CONNECT_FAILED", "connecting", "无法连接 Telnet 主机，请检查地址、端口及网络").with_details(format!("{} ({})", error, error.kind())))?
            }
        };
        let _ = stream.set_nodelay(true);
        let (mut reader, mut writer) = stream.into_split();
        // 所有数据和协商使用同一有界写队列；读写并发，避免大输入阻塞接收。
        let (outgoing, mut packets) = mpsc::channel::<Vec<u8>>(8);
        let mut protocol = protocol::Protocol::new(context.size);
        outgoing
            .send(protocol.initial())
            .await
            .map_err(|_| SessionError::cancelled())?;
        context.events.state(SessionState::Connected)?;
        let write = async {
            while let Some(packet) = packets.recv().await {
                tokio::time::timeout(Duration::from_secs(30), writer.write_all(&packet))
                    .await
                    .map_err(|_| SessionError::new("IO_TIMEOUT", "input", "Telnet 写入超时"))?
                    .map_err(|_| io_error())?;
            }
            Ok::<_, SessionError>(())
        };
        let read = async {
            let mut buffer = [0u8; 8192];
            loop {
                tokio::select! {
                    result = reader.read(&mut buffer) => {
                        let count = result.map_err(|_| io_error())?;
                        if count == 0 { return Ok(SessionEnd::default()); }
                        let (data, reply) = protocol.receive(&buffer[..count])?;
                        if !reply.is_empty() { outgoing.send(reply).await.map_err(|_| SessionError::cancelled())?; }
                        if !data.is_empty() { context.output.send(Bytes::from(data)).await?; }
                    }
                    chunk = context.input.recv() => {
                        let Some(chunk) = chunk else { return Err(SessionError::cancelled()); };
                        outgoing.send(protocol.input(&chunk.data)).await.map_err(|_| SessionError::cancelled())?;
                    }
                    result = context.resize.changed() => {
                        if result.is_err() { return Err(SessionError::cancelled()); }
                        let size = *context.resize.borrow_and_update();
                        let packet = protocol.resize(size);
                        if !packet.is_empty() { outgoing.send(packet).await.map_err(|_| SessionError::cancelled())?; }
                    }
                }
            }
        };
        tokio::select! {
            biased;
            _ = context.cancel.cancelled() => Err(SessionError::cancelled()),
            result = write => { result?; Err(io_error()) },
            result = read => result,
        }
    }
}
fn io_error() -> SessionError {
    SessionError::new("IO_FAILED", "telnet", "Telnet 连接已中断，请重新连接")
}

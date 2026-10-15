//! SSH 适配器：主机身份校验完成前不读取或发送认证秘密。
mod auth;
mod channels;
mod connection;
mod driver;
mod host_key;
pub use channels::SshChannels;

use async_trait::async_trait;
use std::sync::Arc;
use terminal_core::*;

pub struct SshFactory {
    trust: Arc<dyn HostTrustStore>,
    credentials: Arc<dyn CredentialStore>,
    channels: SshChannels,
    files_only: bool,
}
impl SshFactory {
    pub fn new(trust: Arc<dyn HostTrustStore>, credentials: Arc<dyn CredentialStore>) -> Self {
        Self {
            trust,
            credentials,
            channels: SshChannels::default(),
            files_only: false,
        }
    }
    /// 独立文件连接复用认证与子系统注册，不申请PTY或启动Shell。
    pub fn for_files(&self) -> Self {
        Self {
            trust: self.trust.clone(),
            credentials: self.credentials.clone(),
            channels: self.channels.clone(),
            files_only: true,
        }
    }
    pub fn channels(&self) -> SshChannels {
        self.channels.clone()
    }
}
#[async_trait]
impl SessionFactory for SshFactory {
    fn kind(&self) -> &'static str {
        "ssh"
    }
    fn capabilities(&self) -> SessionCapabilities {
        let mut capabilities = SessionCapabilities::ssh();
        if self.files_only {
            capabilities.pty = false;
            capabilities.resize = false;
            capabilities.binary_output = false;
        }
        capabilities
    }
    fn validate(&self, config: &SshConfig) -> Result<()> {
        config.validate()
    }
    async fn run(&self, config: SshConfig, context: DriverContext) -> Result<SessionEnd> {
        context.events.state(SessionState::Connecting)?;
        let (mut connection, _transport) =
            connection::connect(&config, &context, self.trust.clone()).await?;
        context.events.state(SessionState::Authenticating)?;
        auth::authenticate(
            &mut connection,
            &config,
            &context,
            self.credentials.as_ref(),
        )
        .await?;
        context.events.state(SessionState::Opening)?;
        let snapshot = context.events.snapshot();
        let (requests, _registration) =
            self.channels.register(&snapshot.session_id, &context.owner);
        let channels = channels::serve(&connection, requests);
        tokio::pin!(channels);
        if self.files_only {
            context.events.state(SessionState::Connected)?;
            let disconnected = async {
                while !connection.is_closed() {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
                Err(ssh_error("IO_FAILED", "sftp", "SFTP连接已断开，请重新连接"))
            };
            return tokio::select! {
                _ = context.cancel.cancelled() => Err(SessionError::cancelled()),
                result = disconnected => result,
                _ = &mut channels => Err(SessionError::cancelled()),
            };
        }
        tokio::select! {
            result = driver::run(&connection, context) => result,
            _ = &mut channels => Err(SessionError::cancelled()),
        }
    }
}

pub(crate) fn ssh_error(code: &str, phase: &str, message: &str) -> SessionError {
    SessionError::new(code, phase, message)
}
pub(crate) async fn network<T>(
    phase: &str,
    future: impl std::future::Future<Output = std::result::Result<T, russh::Error>>,
) -> Result<T> {
    tokio::time::timeout(std::time::Duration::from_secs(30), future)
        .await
        .map_err(|_| ssh_error("CONNECT_TIMEOUT", phase, "SSH 请求超时"))?
        .map_err(|_| ssh_error("IO_FAILED", phase, "SSH 请求失败；连接可能已中断"))
}

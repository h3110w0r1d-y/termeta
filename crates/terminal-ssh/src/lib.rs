//! SSH 适配器：主机身份校验完成前不读取或发送认证秘密。
mod auth;
mod connection;
mod driver;
mod host_key;

use async_trait::async_trait;
use std::sync::Arc;
use terminal_core::*;

pub struct SshFactory {
    trust: Arc<dyn HostTrustStore>,
    credentials: Arc<dyn CredentialStore>,
}
impl SshFactory {
    pub fn new(trust: Arc<dyn HostTrustStore>, credentials: Arc<dyn CredentialStore>) -> Self {
        Self { trust, credentials }
    }
}
#[async_trait]
impl SessionFactory for SshFactory {
    fn kind(&self) -> &'static str {
        "ssh"
    }
    fn capabilities(&self) -> SessionCapabilities {
        SessionCapabilities::ssh()
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
        driver::run(&mut connection, context).await
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

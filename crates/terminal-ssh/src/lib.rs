//! SSH 适配器：主机身份校验完成前不读取或发送认证秘密。
mod auth;
mod channels;
mod connection;
mod driver;
mod host_key;
mod os_detection;
pub use channels::SshChannels;

use async_trait::async_trait;
use std::sync::Arc;
use terminal_core::*;

pub struct SshFactory {
    trust: Arc<dyn HostTrustStore>,
    credentials: Arc<dyn CredentialStore>,
    channels: SshChannels,
    files_only: bool,
    systems: Option<Arc<dyn RemoteSystemStore>>,
}
impl SshFactory {
    pub fn new(trust: Arc<dyn HostTrustStore>, credentials: Arc<dyn CredentialStore>) -> Self {
        Self {
            trust,
            credentials,
            channels: SshChannels::default(),
            files_only: false,
            systems: None,
        }
    }
    /// 独立文件连接复用认证与子系统注册，不申请PTY或启动Shell。
    pub fn for_files(&self) -> Self {
        Self {
            trust: self.trust.clone(),
            credentials: self.credentials.clone(),
            channels: self.channels.clone(),
            files_only: true,
            systems: None,
        }
    }
    pub fn with_system_detection(mut self, store: Arc<dyn RemoteSystemStore>) -> Self {
        self.systems = Some(store);
        self
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
    fn validate(&self, config: &SessionConfig) -> Result<()> {
        match config {
            SessionConfig::Ssh(config) => config.validate(),
            _ => Err(SessionError::new(
                "UNSUPPORTED_KIND",
                "opening",
                "SSH工厂仅接受SSH配置",
            )),
        }
    }
    async fn run(&self, config: SessionConfig, context: DriverContext) -> Result<SessionEnd> {
        self.validate(&config)?;
        let SessionConfig::Ssh(config) = config else {
            unreachable!()
        };
        context.events.state(SessionState::Connecting)?;
        let (mut connection, _transport, software) =
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
        let events = context.events.clone();
        let cancel = context.cancel.clone();
        // 独立探测与终端驱动并发，完成或失败均不改变正常终端生命周期。
        let detection = async {
            let Some(store) = &self.systems else {
                return;
            };
            let Some(profile_id) = &config.system_profile_id else {
                return;
            };
            // PTY/Shell先建立；不与主终端争抢仅允许一个session的服务端配额。
            loop {
                if events.snapshot().state == SessionState::Connected {
                    break;
                }
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
                }
            }
            let token = events.snapshot().session_id;
            if !matches!(store.claim(profile_id, &token, &config).await, Ok(true)) {
                return;
            }
            let _ = events.emit(EventPayload::HostSystemChanged {
                profile_id: profile_id.clone(),
            });
            let icon = os_detection::detect(&connection, &software, &cancel).await;
            if store
                .complete(profile_id, &token, &config, icon)
                .await
                .is_ok()
            {
                let _ = events.emit(EventPayload::HostSystemChanged {
                    profile_id: profile_id.clone(),
                });
            }
        };
        tokio::pin!(detection);
        let mut detection_done = false;
        let driver = driver::run(&connection, context);
        tokio::pin!(driver);
        loop {
            tokio::select! {
                result = &mut driver => return result,
                _ = &mut detection, if !detection_done => { detection_done = true; },
                _ = &mut channels => return Err(SessionError::cancelled()),
            }
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

use crate::{SerialConfig, TelnetConfig};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionError {
    pub code: String,
    pub phase: String,
    pub retryable: bool,
    pub message: String,
}
impl SessionError {
    pub fn new(code: &str, phase: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            phase: phase.into(),
            retryable: false,
            message: message.into(),
        }
    }
    pub fn cancelled() -> Self {
        Self::new("CANCELLED", "closing", "连接已取消")
    }
}
impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for SessionError {}
pub type Result<T> = std::result::Result<T, SessionError>;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    Created,
    Connecting,
    AwaitingHostKey,
    Authenticating,
    AwaitingAuth,
    Opening,
    Connected,
    Closing,
    Closed,
    Failed,
    Disconnected,
}
impl SessionState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Failed | Self::Disconnected)
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}
impl TerminalSize {
    pub fn validate(self) -> Result<()> {
        if self.cols == 0 || self.cols > 1000 || self.rows == 0 || self.rows > 1000 {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "validation",
                "终端尺寸必须在 1–1000 之间",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCapabilities {
    pub resize: bool,
    pub pty: bool,
    pub binary_output: bool,
    pub serial_control: bool,
}
impl SessionCapabilities {
    pub fn ssh() -> Self {
        Self {
            resize: true,
            pty: true,
            binary_output: true,
            serial_control: false,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub protocol_version: u8,
    pub request_id: String,
    pub session_id: String,
    pub terminal_id: String,
    pub kind: String,
    pub title: String,
    pub state: SessionState,
    pub state_revision: u64,
    pub capabilities: SessionCapabilities,
    /// 本次终端实际采用命令模式；前端不能根据可变的主机草稿推断。
    pub ssh_command: bool,
    pub size: TerminalSize,
}
/// 终端启动方式与认证独立；命令原文交给 SSH 服务端解释。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum SshStartup {
    #[default]
    Shell,
    Exec {
        command: String,
    },
}
impl SshStartup {
    pub fn is_exec(&self) -> bool {
        matches!(self, Self::Exec { .. })
    }
    pub fn validate(&self) -> Result<()> {
        if let Self::Exec { command } = self
            && (command.trim().is_empty() || command.len() > 8192 || command.contains('\0'))
        {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "validation",
                "SSH 启动命令不能为空、不能包含 NUL，且不能超过 8192 UTF-8 字节",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SshConfig {
    /// 仅后端保存主机连接设置；前端不能指定或冒用检测归属。
    #[serde(skip)]
    pub system_profile_id: Option<String>,
    pub config_version: u8,
    pub address: String,
    pub port: u16,
    pub username: String,
    pub auth_plan: Vec<AuthStep>,
    #[serde(default)]
    pub startup: SshStartup,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum AuthStep {
    #[serde(rename_all = "camelCase")]
    Password { credential_id: Option<String> },
    #[serde(rename_all = "camelCase")]
    PrivateKey { credential_id: String },
}
impl SshConfig {
    pub fn validate(&self) -> Result<()> {
        self.startup.validate()?;
        if self.config_version != 1
            || self.port == 0
            || self.address.is_empty()
            || self.address.len() > 253
            || self
                .address
                .chars()
                .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '/' | '\\' | '@'))
            || self.username.is_empty()
            || self.username.len() > 256
            || self.username.chars().any(char::is_control)
            || self.auth_plan.len() != 1
        {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "validation",
                "SSH 地址、用户或认证配置无效",
            ));
        }
        let valid_ref = |value: &str| {
            !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
        };
        for step in &self.auth_plan {
            let valid = match step {
                AuthStep::Password { credential_id } => {
                    credential_id.as_deref().is_none_or(valid_ref)
                }
                AuthStep::PrivateKey { credential_id } => valid_ref(credential_id),
            };
            if !valid {
                return Err(SessionError::new(
                    "INVALID_CONFIG",
                    "validation",
                    "认证资源引用无效",
                ));
            }
        }
        if self.address.contains(':') && self.address.parse::<IpAddr>().is_err() {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "validation",
                "端口必须单独填写；IPv6 不带方括号",
            ));
        }
        Ok(())
    }
    pub fn endpoint(&self) -> HostEndpoint {
        let canonical_host = self
            .address
            .parse::<IpAddr>()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| {
                self.address
                    .to_ascii_lowercase()
                    .trim_end_matches('.')
                    .to_owned()
            });
        HostEndpoint {
            host: self.address.clone(),
            canonical_host,
            port: self.port,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostEndpoint {
    pub host: String,
    pub canonical_host: String,
    pub port: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthPrompt {
    pub text: String,
    pub echo: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostKeyDecision {
    Reject,
    AllowOnce,
    TrustAndSave,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    pub protocol_version: u8,
    pub request_id: String,
    pub session_id: String,
    pub event_seq: u64,
    #[serde(flatten)]
    pub payload: EventPayload,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EventPayload {
    #[serde(rename_all = "camelCase")]
    HostSystemChanged {
        profile_id: String,
    },
    StateChanged {
        snapshot: SessionSnapshot,
    },
    #[serde(rename_all = "camelCase")]
    HostKeyChallenge {
        challenge_id: String,
        endpoint: HostEndpoint,
        algorithm: String,
        fingerprint: String,
        expires_at: u64,
    },
    #[serde(rename_all = "camelCase")]
    AuthChallenge {
        challenge_id: String,
        name: String,
        instructions: String,
        prompts: Vec<AuthPrompt>,
        expires_at: u64,
    },
    #[serde(rename_all = "camelCase")]
    Ended {
        final_state: SessionState,
        reason: String,
        exit_code: Option<u32>,
        exit_signal: Option<String>,
        final_offset: String,
        output_complete: bool,
        error: Option<SessionError>,
    },
}
#[derive(Default)]
pub struct SessionEnd {
    pub exit_code: Option<u32>,
    pub exit_signal: Option<String>,
}

/// 协议配置按类型区分，本地终端不携带远端认证字段。
#[derive(Clone, Debug)]
pub enum SessionConfig {
    Ssh(SshConfig),
    Forwarding {
        ssh: SshConfig,
        rule: crate::ForwardRule,
    },
    Local,
    Serial(SerialConfig),
    Telnet(TelnetConfig),
}
impl SessionConfig {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Ssh(config) => config.validate(),
            Self::Forwarding { ssh, rule } => {
                ssh.validate()?;
                rule.validate()
            }
            Self::Local => Ok(()),
            Self::Serial(config) => config.validate(),
            Self::Telnet(config) => config.validate(),
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Ssh(_) => "ssh",
            Self::Forwarding { .. } => "forwarding",
            Self::Local => "local",
            Self::Serial(_) => "serial",
            Self::Telnet(_) => "telnet",
        }
    }
    pub fn title(&self) -> String {
        match self {
            Self::Ssh(config) => format!("{}@{}", config.username, config.address),
            Self::Forwarding { ssh, .. } => format!("{}@{}", ssh.username, ssh.address),
            Self::Local => "本地终端".into(),
            Self::Serial(config) => config.path.clone(),
            Self::Telnet(config) => config.endpoint(),
        }
    }
}
impl From<SshConfig> for SessionConfig {
    fn from(config: SshConfig) -> Self {
        Self::Ssh(config)
    }
}

use crate::{Result, SessionError};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ForwardMode {
    Local,
    Remote,
    Dynamic,
}

/// 规则只含地址和端口；SSH 身份与秘密始终由保存主机加载。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForwardRule {
    pub mode: ForwardMode,
    pub bind_address: String,
    pub bind_port: u16,
    pub target_address: Option<String>,
    pub target_port: Option<u16>,
}
impl ForwardRule {
    pub fn validate(&self) -> Result<()> {
        let invalid = || {
            SessionError::new(
                "INVALID_CONFIG",
                "forwarding",
                "转发地址或端口无效；监听地址须为 IP，端口须为 1–65535",
            )
        };
        let bind: IpAddr = self.bind_address.parse().map_err(|_| invalid())?;
        if self.bind_port == 0 {
            return Err(invalid());
        }
        if self.mode == ForwardMode::Dynamic {
            // SOCKS5 不提供认证，只向本机开放。
            if !bind.is_loopback() || self.target_address.is_some() || self.target_port.is_some() {
                return Err(SessionError::new(
                    "INVALID_CONFIG",
                    "forwarding",
                    "动态 SOCKS5 只允许本机回环地址，且不能指定固定目标",
                ));
            }
        } else {
            let address = self.target_address.as_deref().ok_or_else(invalid)?;
            if !valid_forward_target(address) || self.target_port.is_none_or(|port| port == 0) {
                return Err(invalid());
            }
        }
        Ok(())
    }
}
pub fn valid_forward_target(address: &str) -> bool {
    !address.is_empty()
        && address.len() <= 253
        && !address.chars().any(|c| {
            c.is_control() || c.is_whitespace() || matches!(c, '/' | '\\' | '@' | '[' | ']')
        })
        && (!address.contains(':') || address.parse::<IpAddr>().is_ok())
}

use crate::{Result, SessionError};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Telnet 登录在终端内完成，不引用 SSH 凭据或主机信任。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TelnetConfig {
    pub config_version: u8,
    pub address: String,
    pub port: u16,
}

impl TelnetConfig {
    pub fn validate(&self) -> Result<()> {
        if self.config_version != 1
            || self.port == 0
            || self.address.is_empty()
            || self.address.len() > 253
            || self.address.chars().any(|c| {
                c.is_control() || c.is_whitespace() || matches!(c, '/' | '\\' | '@' | '[' | ']')
            })
            || (self.address.contains(':') && self.address.parse::<IpAddr>().is_err())
        {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "validation",
                "Telnet 地址或端口无效；IPv6 不带方括号",
            ));
        }
        Ok(())
    }
    pub fn endpoint(&self) -> String {
        if self.address.contains(':') {
            format!("[{}]:{}", self.address, self.port)
        } else {
            format!("{}:{}", self.address, self.port)
        }
    }
}

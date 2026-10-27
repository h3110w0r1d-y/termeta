use crate::{Result, SessionError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SerialConfig {
    pub path: String,
    pub baud_rate: u32,
    pub charset: String,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub flow_control: SerialFlowControl,
    pub parity: SerialParity,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SerialFlowControl {
    None,
    Hardware,
    Software,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SerialParity {
    None,
    Odd,
    Even,
}

impl SerialConfig {
    pub fn validate(&self) -> Result<()> {
        if self.path.is_empty()
            || self.path.len() > 1024
            || self.path.chars().any(char::is_control)
            || !(1..=4_000_000).contains(&self.baud_rate)
            || !(5..=8).contains(&self.data_bits)
            || !matches!(self.stop_bits, 1 | 2)
            || !matches!(
                self.charset.as_str(),
                "UTF-8"
                    | "GBK"
                    | "GB18030"
                    | "Big5"
                    | "Shift_JIS"
                    | "EUC-JP"
                    | "EUC-KR"
                    | "windows-1251"
                    | "windows-1252"
                    | "KOI8-R"
            )
        {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "validation",
                "串口、波特率或高级参数无效",
            ));
        }
        Ok(())
    }
}

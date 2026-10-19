//! 官方账号域：HTTP、加密持久化、短期授权生命周期相互隔离。
pub mod deep_link;
pub mod dto;
pub mod network;
pub mod runtime;
pub mod service;
pub mod storage;
use terminal_core::SessionError;
pub fn failure(code: &str, message: &str) -> SessionError {
    SessionError::new(code, "account", message)
}

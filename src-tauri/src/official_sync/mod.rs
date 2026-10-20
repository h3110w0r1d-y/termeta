//! 官方模式按运行状态、HTTP、密码门禁、同步与改密分责，第三方备份独立。
mod envelope;
mod network;
mod password;
mod runtime;
mod service;
mod sync;
pub use password::change_password;
pub use runtime::{Runtime, Status};
pub use service::{complete, status, sync};

//! 前端只读取轻量状态；密钥与快照不进入此摘要。
use serde::Serialize;
use std::sync::Mutex;
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub phase: &'static str,
    pub cloud_has_data: bool,
    pub last_synced_at: Option<String>,
    pub error: Option<String>,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            phase: "signedOut",
            cloud_has_data: false,
            last_synced_at: None,
            error: None,
        }
    }
}
#[derive(Default)]
pub struct Runtime {
    pub operation: tokio::sync::Mutex<()>,
    status: Mutex<Status>,
    pub(super) revision: std::sync::atomic::AtomicI64,
    pub changed: std::sync::atomic::AtomicBool,
    /// 新登录必须显式完成密码门禁；已确认会话可在重启后复用本机主密钥。
    pub(super) requires_password: std::sync::atomic::AtomicBool,
}
impl Runtime {
    pub fn reset(&self) {
        *self.status.lock().unwrap() = Status::default();
        self.revision.store(0, std::sync::atomic::Ordering::Release);
        self.changed
            .store(false, std::sync::atomic::Ordering::Release);
        self.requires_password
            .store(true, std::sync::atomic::Ordering::Release);
    }
    pub fn value(&self) -> Status {
        self.status.lock().unwrap().clone()
    }
    pub(super) fn set(&self, value: Status) -> Status {
        *self.status.lock().unwrap() = value.clone();
        value
    }
    pub fn ready(&self) -> bool {
        self.value().phase == "ready"
    }
}

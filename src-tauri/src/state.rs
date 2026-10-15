use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tauri::{
    WebviewWindow,
    ipc::{Channel, Response},
};
use terminal_core::{Result, SessionError, SessionEvent, SessionManager, SessionSink};
use terminal_security::CredentialVault;
use terminal_storage::SqliteStore;

pub struct AppState {
    pub sessions: SessionManager,
    pub store: Arc<SqliteStore>,
    pub vault: Arc<CredentialVault>,
    pub heartbeat: Mutex<Instant>,
    pub open_gates:
        Mutex<std::collections::HashMap<String, std::sync::Weak<tokio::sync::Mutex<()>>>>,
}

pub fn owner(window: &WebviewWindow) -> Result<String> {
    if window.label() != "main" || !crate::local_navigation(&window.url().map_err(|_| forbidden())?)
    {
        return Err(forbidden());
    }
    Ok(window.label().to_owned())
}
fn forbidden() -> SessionError {
    SessionError::new("FORBIDDEN", "ownership", "此窗口无权访问桌面资源")
}
pub fn storage_error(error: terminal_storage::StorageError) -> SessionError {
    use terminal_storage::StorageError;
    let code = match &error {
        StorageError::Conflict => "REVISION_CONFLICT",
        StorageError::NotFound => "NOT_FOUND",
        StorageError::Validation(_) | StorageError::Json(_) => "INVALID_CONFIG",
        _ => "STORAGE_UNAVAILABLE",
    };
    SessionError::new(code, "storage", &error.to_string())
}

pub struct DesktopSink {
    pub output: Channel<Response>,
    pub control: Channel<SessionEvent>,
    pub store: Arc<SqliteStore>,
    pub history_id: String,
}
impl SessionSink for DesktopSink {
    fn output(&self, frame: Vec<u8>) -> Result<()> {
        self.output
            .send(Response::new(frame))
            .map_err(|_| SessionError::new("CLIENT_DISCONNECTED", "output", "终端输出接收端已关闭"))
    }
    fn event(&self, event: SessionEvent) -> Result<()> {
        if let terminal_core::EventPayload::Ended { reason, error, .. } = &event.payload {
            let store = self.store.clone();
            let id = self.history_id.clone();
            let reason = reason.clone();
            let code = error.as_ref().map(|error| error.code.clone());
            tauri::async_runtime::spawn(async move {
                // 连接历史失败不阻塞终态，也不把认证或输出正文写入诊断。
                let _ = store.end_history(&id, &reason, code.as_deref()).await;
            });
        }
        self.control.send(event).map_err(|_| {
            SessionError::new("CLIENT_DISCONNECTED", "control", "终端控制接收端已关闭")
        })
    }
}

impl AppState {
    pub fn request_gate(&self, request_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut gates = self.open_gates.lock().unwrap();
        gates.retain(|_, gate| gate.strong_count() > 0);
        if let Some(gate) = gates.get(request_id).and_then(std::sync::Weak::upgrade) {
            return gate;
        }
        let gate = Arc::new(tokio::sync::Mutex::new(()));
        gates.insert(request_id.to_owned(), Arc::downgrade(&gate));
        gate
    }
}

use std::sync::{Arc, Mutex};
use tauri::{
    WebviewWindow,
    ipc::{Channel, Response},
};
use terminal_core::{Result, SessionError, SessionEvent, SessionManager, SessionSink};
use terminal_security::CredentialVault;
use terminal_storage::SqliteStore;

pub struct AppState {
    pub official: crate::official_sync::Runtime,
    pub account: crate::account::runtime::AccountRuntime,
    pub backup: crate::backup_runtime::BackupRuntime,
    pub settings_gate: tokio::sync::Mutex<()>,
    pub sessions: SessionManager,
    pub file_sessions: SessionManager,
    pub forward_sessions: SessionManager,
    pub ssh_channels: terminal_ssh::SshChannels,
    pub sftp: SftpBrowsers,
    pub transfers: Arc<tokio::sync::Semaphore>,
    pub store: Arc<SqliteStore>,
    pub vault: Arc<CredentialVault>,
    pub open_gates:
        Mutex<std::collections::HashMap<String, std::sync::Weak<tokio::sync::Mutex<()>>>>,
}

pub type SftpBrowsers =
    Arc<Mutex<std::collections::HashMap<String, Arc<terminal_sftp::SftpBrowser>>>>;

/// 三类 manager 共用资源撤销入口，但仍各自校验 owner、用途与会话预算。
pub fn sftp_resource_cleanup(browsers: SftpBrowsers) -> Arc<terminal_core::SessionResourceCleanup> {
    Arc::new(move |_owner, id| {
        let browser = browsers.lock().unwrap().remove(id);
        // 网络关闭与任务取消在全局资源表锁外执行。
        if let Some(browser) = browser {
            browser.cancel();
        }
    })
}

pub fn window_owner(window: &WebviewWindow) -> Result<String> {
    if window.label() != "main" || !crate::local_navigation(&window.url().map_err(|_| forbidden())?)
    {
        return Err(forbidden());
    }
    Ok(window.label().to_owned())
}
pub fn owner(window: &WebviewWindow) -> Result<String> {
    use tauri::Manager;
    let id = window_owner(window)?;
    if !window.state::<AppState>().vault.is_unlocked() {
        return Err(SessionError::new(
            "VAULT_LOCKED",
            "storage",
            "请先解锁数据保险库",
        ));
    }
    Ok(id)
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
}
impl SessionSink for DesktopSink {
    fn output(&self, frame: Vec<u8>) -> Result<()> {
        self.output
            .send(Response::new(frame))
            .map_err(|_| SessionError::new("CLIENT_DISCONNECTED", "output", "终端输出接收端已关闭"))
    }
    fn event(&self, event: SessionEvent) -> Result<()> {
        self.control.send(event).map_err(|_| {
            SessionError::new("CLIENT_DISCONNECTED", "control", "终端控制接收端已关闭")
        })
    }
}

impl AppState {
    /// 定时扫描仅兜底；正常结束、显式关闭与 owner 销毁均由 core 同步通知撤销。
    pub fn cleanup_sftp(&self) {
        let ids: Vec<_> = self.sftp.lock().unwrap().keys().cloned().collect();
        let cleanup = sftp_resource_cleanup(self.sftp.clone());
        for id in ids {
            let alive = self
                .session_manager("main", &id)
                .and_then(|manager| manager.resource_cancellation("main", &id))
                .is_ok_and(|cancel| !cancel.is_cancelled());
            if !alive {
                cleanup("main", &id);
            }
        }
    }
    /// 资源归属仍由各会话表校验，文件连接与终端连接共用认证响应入口。
    pub fn session_manager(&self, owner: &str, id: &str) -> Result<&SessionManager> {
        if self.sessions.get(owner, id).is_ok() {
            return Ok(&self.sessions);
        }
        if self.file_sessions.get(owner, id).is_ok() {
            return Ok(&self.file_sessions);
        }
        self.forward_sessions.get(owner, id)?;
        Ok(&self.forward_sessions)
    }

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

// 设置窗口仅供明确授权的设置与维护命令使用，不能获得会话所有权。
pub fn settings_owner(window: &WebviewWindow) -> Result<()> {
    use tauri::Manager;
    if !matches!(window.label(), "main" | "settings")
        || !crate::local_navigation(&window.url().map_err(|_| forbidden())?)
    {
        return Err(forbidden());
    }
    if !window.state::<AppState>().vault.is_unlocked() {
        return Err(SessionError::new(
            "VAULT_LOCKED",
            "storage",
            "请先解锁数据保险库",
        ));
    }
    Ok(())
}

/// 启动登录只允许本地 main；已解锁设置窗口也可管理账号。
pub fn account_owner(window: &WebviewWindow) -> Result<()> {
    if window.label() == "main" {
        window_owner(window).map(|_| ())
    } else {
        settings_owner(window)
    }
}

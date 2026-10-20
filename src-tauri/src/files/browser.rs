use crate::state::AppState;
use std::sync::Arc;
use terminal_core::{Result, SessionState};
use terminal_files::file_error;
use terminal_sftp::SftpBrowser;

pub(crate) fn connected(state: &AppState, owner: &str, id: &str) -> Result<()> {
    let manager = state.session_manager(owner, id)?;
    if manager.get(owner, id)?.state != SessionState::Connected
        || manager.resource_cancellation(owner, id)?.is_cancelled()
    {
        return Err(file_error("SSH 已断开，文件列表已过期，请重连后刷新"));
    }
    Ok(())
}
pub(crate) async fn browser(state: &AppState, owner: &str, id: &str) -> Result<Arc<SftpBrowser>> {
    connected(state, owner, id)?;
    let gate = state.request_gate(&format!("sftp:{id}"));
    let _gate = gate.lock().await;
    connected(state, owner, id)?;
    let existing = state.sftp.lock().unwrap().get(id).cloned();
    if let Some(browser) = existing {
        return Ok(browser);
    }
    let lifetime = state
        .session_manager(owner, id)?
        .resource_cancellation(owner, id)?;
    let browser = tokio::select! {
        biased;
        _ = lifetime.cancelled() => return Err(terminal_core::SessionError::cancelled()),
        browser = SftpBrowser::connect(&state.ssh_channels, owner, id, lifetime.child_token()) => Arc::new(browser?),
    };
    // 和撤销回调使用同一张表；锁内检查代次令牌，防止刚清理完又插回迟到浏览器。
    let mut browsers = state.sftp.lock().unwrap();
    if lifetime.is_cancelled() {
        return Err(terminal_core::SessionError::cancelled());
    }
    browsers.insert(id.into(), browser.clone());
    Ok(browser)
}

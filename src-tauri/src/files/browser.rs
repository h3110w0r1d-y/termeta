use crate::state::AppState;
use std::sync::Arc;
use terminal_core::{Result, SessionState};
use terminal_files::file_error;
use terminal_sftp::SftpBrowser;

pub(crate) fn connected(state: &AppState, owner: &str, id: &str) -> Result<()> {
    if state.session_manager(owner, id)?.get(owner, id)?.state != SessionState::Connected {
        return Err(file_error("SSH 已断开，文件列表已过期，请重连后刷新"));
    }
    Ok(())
}
pub(crate) async fn browser(state: &AppState, owner: &str, id: &str) -> Result<Arc<SftpBrowser>> {
    connected(state, owner, id)?;
    let gate = state.request_gate(&format!("sftp:{id}"));
    let _gate = gate.lock().await;
    let existing = state.sftp.lock().unwrap().get(id).cloned();
    if let Some(browser) = existing {
        return Ok(browser);
    }
    let browser = Arc::new(SftpBrowser::connect(&state.ssh_channels, owner, id).await?);
    connected(state, owner, id)?;
    state
        .sftp
        .lock()
        .unwrap()
        .insert(id.into(), browser.clone());
    Ok(browser)
}

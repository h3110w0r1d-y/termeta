use crate::state::{AppState, owner, storage_error};
use std::sync::Arc;
use tauri::{State, WebviewWindow, ipc::Channel};
use terminal_core::*;
use terminal_storage::{ForwardDraft, ForwardProfile};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardList {
    profiles: Vec<ForwardProfile>,
    sessions: Vec<SessionSnapshot>,
}
struct ForwardSink(Channel<SessionEvent>);
impl SessionSink for ForwardSink {
    fn output(&self, _: Vec<u8>) -> Result<()> {
        Err(SessionError::new(
            "UNSUPPORTED_OPERATION",
            "forwarding",
            "转发不产生终端输出",
        ))
    }
    fn event(&self, event: SessionEvent) -> Result<()> {
        self.0.send(event).map_err(|_| {
            SessionError::new("CLIENT_DISCONNECTED", "forwarding", "转发控制接收端已关闭")
        })
    }
}
fn active(state: &AppState, owner: &str, id: &str) -> Option<SessionSnapshot> {
    state
        .forward_sessions
        .list(owner)
        .into_iter()
        .find(|s| s.terminal_id == id && !s.state.terminal())
}
fn busy() -> SessionError {
    SessionError::new(
        "FORWARD_ACTIVE",
        "forwarding",
        "请先停止端口转发，再编辑或删除配置",
    )
}
#[tauri::command]
pub async fn list_port_forwards(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<ForwardList> {
    let owner = owner(&window)?;
    Ok(ForwardList {
        profiles: state.store.list_forwards().await.map_err(storage_error)?,
        sessions: state.forward_sessions.list(&owner),
    })
}
#[tauri::command]
pub async fn save_port_forward(
    window: WebviewWindow,
    state: State<'_, AppState>,
    draft: ForwardDraft,
    expected_revision: Option<i64>,
) -> Result<ForwardProfile> {
    let owner = owner(&window)?;
    let gate = state.request_gate("forwarding-management");
    let _gate = gate.lock().await;
    if draft
        .id
        .as_deref()
        .is_some_and(|id| active(&state, &owner, id).is_some())
    {
        return Err(busy());
    }
    state
        .store
        .save_forward(draft, expected_revision)
        .await
        .map_err(storage_error)
}
#[tauri::command]
pub async fn delete_port_forward(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
    revision: i64,
) -> Result<()> {
    let owner = owner(&window)?;
    let gate = state.request_gate("forwarding-management");
    let _gate = gate.lock().await;
    if active(&state, &owner, &id).is_some() {
        return Err(busy());
    }
    state
        .store
        .delete_forward(&id, revision)
        .await
        .map_err(storage_error)
}
#[tauri::command]
pub async fn start_port_forward(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
    request_id: String,
    on_control: Channel<SessionEvent>,
) -> Result<SessionSnapshot> {
    let owner = owner(&window)?;
    if uuid::Uuid::parse_str(&request_id).is_err() {
        return Err(SessionError::new(
            "INVALID_CONFIG",
            "forwarding",
            "请求标识无效",
        ));
    }
    let gate = state.request_gate("forwarding-management");
    let _gate = gate.lock().await;
    if let Some(snapshot) = state.forward_sessions.lookup_request(&owner, &request_id) {
        if snapshot.terminal_id != id {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "forwarding",
                "请求标识已用于其他转发",
            ));
        }
        return Ok(snapshot);
    }
    if let Some(snapshot) = active(&state, &owner, &id) {
        return Ok(snapshot);
    }
    let profile = state.store.get_forward(&id).await.map_err(storage_error)?;
    let host = profile.profile_id.ok_or_else(|| {
        SessionError::new(
            "NOT_FOUND",
            "forwarding",
            "引用的主机已删除，请编辑并重新选择 SSH 主机",
        )
    })?;
    let mut ssh = state
        .store
        .get_host_connection(&host)
        .await
        .map_err(storage_error)?;
    // 转发不会启动终端程序，保存主机的命令模式不参与其生命周期。
    ssh.startup = terminal_core::SshStartup::default();
    state.forward_sessions.open(
        &owner,
        request_id,
        id,
        SessionConfig::Forwarding {
            ssh,
            rule: profile.rule,
        },
        TerminalSize { cols: 80, rows: 24 },
        Arc::new(ForwardSink(on_control)),
    )
}
#[tauri::command]
pub async fn stop_port_forward(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<()> {
    let owner = owner(&window)?;
    // 与启动串行；按配置标识停止，覆盖尚未返回 sessionId 的建立阶段。
    let gate = state.request_gate("forwarding-management");
    let _gate = gate.lock().await;
    if let Some(snapshot) = active(&state, &owner, &id) {
        state.forward_sessions.close(&owner, &snapshot.session_id)?;
    }
    Ok(())
}

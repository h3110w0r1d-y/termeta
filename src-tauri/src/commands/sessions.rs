use crate::state::{AppState, DesktopSink, owner, storage_error};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{
    State, WebviewWindow,
    ipc::{Channel, Response},
};
use terminal_core::*;

#[derive(Deserialize)]
#[serde(tag = "source", rename_all = "camelCase", deny_unknown_fields)]
pub enum OpenTarget {
    #[serde(rename_all = "camelCase")]
    Saved {
        profile_id: String,
    },
    Ephemeral {
        kind: String,
        config: SshConfig,
    },
}
#[tauri::command]
pub fn available_session_kinds(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<serde_json::Value> {
    owner(&window)?;
    Ok(serde_json::json!([{"kind":"ssh","capabilities":state.sessions.capabilities()}]))
}
#[tauri::command]
pub async fn open_session(
    window: WebviewWindow,
    state: State<'_, AppState>,
    protocol_version: u8,
    request_id: String,
    terminal_id: String,
    target: OpenTarget,
    size: TerminalSize,
    on_output: Channel<Response>,
    on_control: Channel<SessionEvent>,
) -> Result<SessionSnapshot> {
    let owner = owner(&window)?;
    if uuid::Uuid::parse_str(&request_id).is_err() || uuid::Uuid::parse_str(&terminal_id).is_err() {
        return Err(SessionError::new(
            "INVALID_CONFIG",
            "validation",
            "请求与终端标识必须为 UUID",
        ));
    }
    let gate = state.request_gate(&request_id);
    let _gate = gate.lock().await;
    if protocol_version != 1 {
        return Err(SessionError::new(
            "PROTOCOL_MISMATCH",
            "opening",
            "不支持的终端协议版本",
        ));
    }
    if let Some(snapshot) = state.sessions.lookup_request(&owner, &request_id) {
        return Ok(snapshot);
    }
    let (config, profile_id, name) = match target {
        OpenTarget::Saved { profile_id } => {
            let host = state
                .store
                .get_host(&profile_id)
                .await
                .map_err(storage_error)?;
            let config = state
                .store
                .get_host_connection(&profile_id)
                .await
                .map_err(storage_error)?;
            (config, Some(profile_id), host.name)
        }
        OpenTarget::Ephemeral { kind, config } => {
            if kind != "ssh" {
                return Err(SessionError::new(
                    "UNSUPPORTED_KIND",
                    "opening",
                    "首期仅提供 SSH 连接",
                ));
            }
            // 临时连接只能引用公共库，不能冒用任何主机的专属秘密。
            for step in &config.auth_plan {
                let id = match step {
                    AuthStep::Password { credential_id } => credential_id.as_deref(),
                    AuthStep::PrivateKey { credential_id } => Some(credential_id.as_str()),
                };
                if let Some(id) = id {
                    let credential = state
                        .store
                        .get_credential(id)
                        .await
                        .map_err(storage_error)?;
                    let expected_kind = if matches!(step, AuthStep::Password { .. }) {
                        "password"
                    } else {
                        "privateKey"
                    };
                    if credential.owner_profile_id.is_some()
                        || credential.kind != expected_kind
                        || credential.status != "ready"
                    {
                        return Err(SessionError::new(
                            "INVALID_CONFIG",
                            "validation",
                            "临时连接只能引用匹配的公共凭据",
                        ));
                    }
                }
            }
            let name = format!("{}@{}", config.username, config.address);
            (config, None, name)
        }
    };
    config.validate()?;
    size.validate()?;
    // 历史记录只存连接摘要；requestId 在同一窗口内同时承担幂等键。
    let endpoint = if config.address.contains(':') {
        format!("[{}]:{}", config.address, config.port)
    } else {
        format!("{}:{}", config.address, config.port)
    };
    let history_id = state
        .store
        .start_history(profile_id.as_deref(), &name, Some(&endpoint))
        .await
        .map_err(storage_error)?;
    let sink = Arc::new(DesktopSink {
        output: on_output,
        control: on_control,
        store: state.store.clone(),
        history_id: history_id.clone(),
    });
    match state
        .sessions
        .open(&owner, request_id, terminal_id, config, size, sink)
    {
        Ok(snapshot) => Ok(snapshot),
        Err(error) => {
            let _ = state
                .store
                .end_history(&history_id, "openRejected", Some(&error.code))
                .await;
            Err(error)
        }
    }
}
#[tauri::command]
pub async fn write_session(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    input_seq: u32,
    bytes: Vec<u8>,
) -> Result<serde_json::Value> {
    let accepted = state
        .sessions
        .write(&owner(&window)?, &session_id, input_seq, bytes)
        .await?;
    Ok(serde_json::json!({"acceptedSeq":accepted}))
}
#[tauri::command]
pub fn resize_session(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    size: TerminalSize,
) -> Result<TerminalSize> {
    state.sessions.resize(&owner(&window)?, &session_id, size)
}
#[tauri::command]
pub fn ack_session_output(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    consumed_offset: String,
) -> Result<()> {
    if consumed_offset.len() > 20
        || consumed_offset.is_empty()
        || !consumed_offset.bytes().all(|c| c.is_ascii_digit())
    {
        return Err(SessionError::new(
            "PROTOCOL_MISMATCH",
            "output",
            "非法累计输出偏移",
        ));
    }
    let offset = consumed_offset
        .parse()
        .map_err(|_| SessionError::new("PROTOCOL_MISMATCH", "output", "输出偏移超出范围"))?;
    state.sessions.ack(&owner(&window)?, &session_id, offset)
}
#[tauri::command]
pub fn respond_host_key(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    challenge_id: String,
    decision: HostKeyDecision,
) -> Result<()> {
    state
        .sessions
        .respond_host_key(&owner(&window)?, &session_id, &challenge_id, decision)
}
#[tauri::command]
pub fn respond_auth_challenge(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    challenge_id: String,
    answers: Vec<String>,
    save_policy: String,
) -> Result<()> {
    if save_policy != "never" {
        return Err(SessionError::new(
            "UNSUPPORTED_SAVE_POLICY",
            "authentication",
            "当前认证只允许本次使用",
        ));
    }
    state
        .sessions
        .respond_auth(&owner(&window)?, &session_id, &challenge_id, answers)
}
#[tauri::command]
pub fn cancel_auth_challenge(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    challenge_id: String,
) -> Result<SessionSnapshot> {
    // challengeId 必须匹配当前活动挑战，不能取消旧弹窗所指代的新连接。
    state
        .sessions
        .cancel_auth(&owner(&window)?, &session_id, &challenge_id)?;
    state.sessions.close(&owner(&window)?, &session_id)
}
#[tauri::command]
pub fn close_session(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    reason: String,
) -> Result<SessionSnapshot> {
    if !matches!(
        reason.as_str(),
        "user"
            | "tabClosed"
            | "windowClosed"
            | "reconnect"
            | "protocolError"
            | "protocolMismatch"
            | "inputFailed"
            | "disconnect"
    ) {
        return Err(SessionError::new(
            "INVALID_CONFIG",
            "closing",
            "未知关闭原因",
        ));
    }
    let snapshot = state
        .sessions
        .close_with_reason(&owner(&window)?, &session_id, &reason)?;
    // 关闭SSH时立即取消等待选择器/覆盖确认/原生文件承诺，不等待维护周期。
    if let Some(browser) = state.sftp.lock().unwrap().remove(&session_id) {
        browser.cancel();
    }
    Ok(snapshot)
}
#[tauri::command]
pub fn list_sessions(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<SessionSnapshot>> {
    Ok(state.sessions.list(&owner(&window)?))
}
#[tauri::command]
pub fn get_session(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<SessionSnapshot> {
    state.sessions.get(&owner(&window)?, &session_id)
}
#[tauri::command]
pub fn session_client_heartbeat(window: WebviewWindow, state: State<'_, AppState>) -> Result<()> {
    owner(&window)?;
    *state.heartbeat.lock().unwrap() = std::time::Instant::now();
    Ok(())
}

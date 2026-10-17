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
    Local,
    Telnet {
        config: TelnetConfig,
    },
    Serial {
        config: SerialConfig,
    },
    Ephemeral {
        kind: String,
        config: SshConfig,
    },
}
#[tauri::command]
pub async fn list_serial_ports(
    window: WebviewWindow,
) -> Result<Vec<terminal_serial::SerialPortInfo>> {
    owner(&window)?;
    tokio::task::spawn_blocking(terminal_serial::list_ports)
        .await
        .map_err(|_| SessionError::new("SERIAL_ENUM_FAILED", "serial", "无法枚举串口"))?
}
#[tauri::command]
pub fn available_session_kinds(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<serde_json::Value> {
    owner(&window)?;
    Ok(serde_json::json!(
        state
            .sessions
            .available_kinds()
            .into_iter()
            .map(
                |(kind, capabilities)| serde_json::json!({"kind":kind,"capabilities":capabilities})
            )
            .collect::<Vec<_>>()
    ))
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
    let config = match target {
        OpenTarget::Local => SessionConfig::Local,
        OpenTarget::Telnet { config } => SessionConfig::Telnet(config),
        OpenTarget::Serial { config } => SessionConfig::Serial(config),
        OpenTarget::Saved { profile_id } => {
            let host = state
                .store
                .get_host(&profile_id)
                .await
                .map_err(storage_error)?;
            let config = match host.kind.as_str() {
                "ssh" => {
                    let mut config = state
                        .store
                        .get_host_connection(&profile_id)
                        .await
                        .map_err(storage_error)?;
                    config.system_profile_id = Some(profile_id.clone());
                    SessionConfig::Ssh(config)
                }
                "telnet" => SessionConfig::Telnet(TelnetConfig {
                    config_version: 1,
                    address: host.address,
                    port: host.port,
                }),
                "serial" => SessionConfig::Serial(host.serial_config.ok_or_else(|| {
                    SessionError::new("INVALID_CONFIG", "validation", "串口主机缺少串口配置")
                })?),
                _ => {
                    return Err(SessionError::new(
                        "UNSUPPORTED_KIND",
                        "opening",
                        "不支持的主机类型",
                    ));
                }
            };
            config
        }
        OpenTarget::Ephemeral { kind, config } => {
            if kind != "ssh" {
                return Err(SessionError::new(
                    "UNSUPPORTED_KIND",
                    "opening",
                    "临时远端连接仅支持 SSH，本地终端请使用 local 目标",
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
            SessionConfig::Ssh(config)
        }
    };
    config.validate()?;
    size.validate()?;
    let sink = Arc::new(DesktopSink {
        output: on_output,
        control: on_control,
    });
    state
        .sessions
        .open(&owner, request_id, terminal_id, config, size, sink)
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
    let owner = owner(&window)?;
    state
        .session_manager(&owner, &session_id)?
        .respond_host_key(&owner, &session_id, &challenge_id, decision)
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
    let owner = owner(&window)?;
    state.session_manager(&owner, &session_id)?.respond_auth(
        &owner,
        &session_id,
        &challenge_id,
        answers,
    )
}
#[tauri::command]
pub fn cancel_auth_challenge(
    window: WebviewWindow,
    state: State<'_, AppState>,
    session_id: String,
    challenge_id: String,
) -> Result<SessionSnapshot> {
    // challengeId 必须匹配当前活动挑战，不能取消旧弹窗所指代的新连接。
    let owner = owner(&window)?;
    let manager = state.session_manager(&owner, &session_id)?;
    manager.cancel_auth(&owner, &session_id, &challenge_id)?;
    manager.close(&owner, &session_id)
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
    let owner = owner(&window)?;
    let snapshot = state
        .session_manager(&owner, &session_id)?
        .close_with_reason(&owner, &session_id, &reason)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn telnet_target_is_typed_and_rejects_ssh_authentication_fields() {
        let target = serde_json::json!({"source":"telnet","config":{"configVersion":1,"address":"::1","port":23}});
        assert!(matches!(
            serde_json::from_value::<OpenTarget>(target.clone()).unwrap(),
            OpenTarget::Telnet { .. }
        ));
        for field in ["username", "authPlan", "serialConfig", "systemProfileId"] {
            let mut invalid = target.clone();
            invalid["config"][field] = serde_json::json!("forged");
            assert!(serde_json::from_value::<OpenTarget>(invalid).is_err());
        }
        let mut invalid = target;
        invalid["password"] = serde_json::json!("forged");
        assert!(serde_json::from_value::<OpenTarget>(invalid).is_err());
    }
}

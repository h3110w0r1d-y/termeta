use crate::{host_key::HostKeyHandler, network, ssh_error};
use russh::{Channel, ChannelMsg, client};
use std::{collections::VecDeque, time::Duration};
use terminal_core::*;

pub(crate) async fn run(
    connection: &mut client::Handle<HostKeyHandler>,
    mut context: DriverContext,
) -> Result<SessionEnd> {
    let mut channel = network("opening", connection.channel_open_session()).await?;
    let mut early = VecDeque::new();
    let size = *context.resize.borrow_and_update();
    channel
        .request_pty(
            true,
            "xterm-256color",
            size.cols.into(),
            size.rows.into(),
            0,
            0,
            &[],
        )
        .await
        .map_err(|_| ssh_error("PTY_REJECTED", "opening", "无法发送 PTY 请求"))?;
    accepted(&mut channel, "PTY_REJECTED", &mut early).await?;
    channel
        .request_shell(true)
        .await
        .map_err(|_| ssh_error("SHELL_REJECTED", "opening", "无法发送 Shell 请求"))?;
    accepted(&mut channel, "SHELL_REJECTED", &mut early).await?;
    context.events.state(SessionState::Connected)?;
    let (mut reader, writer) = channel.split();
    // 写入等待远端窗口时，读取与输出仍由外层循环驱动，避免双向高流量互相等待。
    let controls = control_loop(writer, context.input, context.resize);
    tokio::pin!(controls);
    let mut end = SessionEnd::default();
    let mut pending = early.pop_front();
    loop {
        // 输出信用耗尽时不再从 SSH 取输出，但输入、resize 和独立取消仍有机会被处理。
        tokio::select! {
            _ = context.cancel.cancelled() => return Err(SessionError::cancelled()),
            result = async { context.output.send(pending.as_ref().unwrap().clone()).await }, if pending.is_some() => {
                result?;
                pending = early.pop_front();
            }
            result = &mut controls => return result.map(|()| end),
            message = reader.wait(), if pending.is_none() => {
                match message {
                    Some(ChannelMsg::Data { data }) | Some(ChannelMsg::ExtendedData { data, .. }) => {
                        if !data.is_empty() {
                            for chunk in data.chunks(MAX_OUTPUT_PAYLOAD) { early.push_back(Bytes::copy_from_slice(chunk)); }
                            pending = early.pop_front();
                        }
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => end.exit_code = Some(exit_status),
                    Some(ChannelMsg::ExitSignal { signal_name, .. }) => end.exit_signal = Some(match signal_name {
                        russh::Sig::Custom(name) => name.chars().filter(|character| !character.is_control()).take(128).collect(),
                        known => format!("{known:?}"),
                    }),
                    Some(ChannelMsg::Eof) => {},
                    Some(ChannelMsg::Close) => {
                        let _ = tokio::time::timeout(Duration::from_secs(1), connection.disconnect(russh::Disconnect::ByApplication, "", "")).await;
                        return Ok(end);
                    }
                    None => return if end.exit_code.is_some() || end.exit_signal.is_some() { Ok(end) } else { Err(ssh_error("IO_FAILED", "connected", "SSH 连接意外中断，终端历史已保留")) },
                    _ => {},
                }
            }
        }
    }
}
async fn control_loop(
    writer: russh::ChannelWriteHalf<client::Msg>,
    mut input: tokio::sync::mpsc::Receiver<InputChunk>,
    mut resize: tokio::sync::watch::Receiver<TerminalSize>,
) -> Result<()> {
    loop {
        tokio::select! {
            changed = resize.changed() => {
                changed.map_err(|_| SessionError::cancelled())?;
                let size = *resize.borrow_and_update();
                network("resize", writer.window_change(size.cols.into(), size.rows.into(), 0, 0)).await?;
            }
            chunk = input.recv() => {
                let Some(chunk) = chunk else { return Err(SessionError::cancelled()); };
                // 超时或失败不能确定远端已接收多少；保留字节配额直到本次写入结束，绝不自动重发。
                let result = network("input", writer.data_bytes(chunk.data.clone())).await;
                drop(chunk);
                if result.is_err() { return Err(ssh_error("INPUT_DELIVERY_UNKNOWN", "input", "输入交付状态不确定，请勿自动重发")); }
            }
        }
    }
}

async fn accepted(
    channel: &mut Channel<client::Msg>,
    code: &str,
    early: &mut VecDeque<Bytes>,
) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut total: usize = early.iter().map(Bytes::len).sum();
        loop {
            match channel.wait().await {
                Some(ChannelMsg::Success) => return Ok(()),
                Some(ChannelMsg::Failure | ChannelMsg::Close) | None => {
                    return Err(ssh_error(code, "opening", "服务器拒绝终端请求"));
                }
                Some(ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. }) => {
                    total += data.len();
                    if total > 262144 {
                        return Err(ssh_error(
                            "LIMIT_EXCEEDED",
                            "opening",
                            "打开终端前的输出超过限制",
                        ));
                    }
                    if !data.is_empty() {
                        for chunk in data.chunks(MAX_OUTPUT_PAYLOAD) {
                            early.push_back(Bytes::copy_from_slice(chunk));
                        }
                    }
                }
                _ => {}
            }
        }
    })
    .await
    .map_err(|_| ssh_error(code, "opening", "等待服务器接受终端请求超时"))?
}

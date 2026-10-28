use crate::files::{TransferRegistration, browser, connected};
use crate::state::{AppState, owner, storage_error};
use std::sync::Arc;
use tauri::{State, WebviewWindow, ipc::Channel};
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};
use terminal_core::{Result, SessionEvent, SessionSink, SessionSnapshot, TerminalSize};
use terminal_files::{TransferProgress, child_path, file_error};

struct FileSink(Channel<SessionEvent>);
impl SessionSink for FileSink {
    fn output(&self, _frame: Vec<u8>) -> Result<()> {
        Err(file_error("文件连接不接收终端输出"))
    }
    fn event(&self, event: SessionEvent) -> Result<()> {
        self.0
            .send(event)
            .map_err(|_| file_error("文件连接控制通道已关闭"))
    }
}

#[tauri::command]
pub async fn sftp_connect(
    window: WebviewWindow,
    state: State<'_, AppState>,
    profile_id: String,
    request_id: String,
    on_control: Channel<SessionEvent>,
) -> Result<SessionSnapshot> {
    let owner = owner(&window)?;
    if uuid::Uuid::parse_str(&request_id).is_err() {
        return Err(file_error("连接请求标识无效"));
    }
    let gate = state.request_gate("sftp-connect");
    let _gate = gate.lock().await;
    if let Some(snapshot) = state.file_sessions.lookup_request(&owner, &request_id) {
        return Ok(snapshot);
    }
    if state
        .file_sessions
        .list(&owner)
        .iter()
        .filter(|session| !session.state.terminal())
        .count()
        >= 2
    {
        return Err(file_error("双栏最多建立两个独立SFTP连接，请先断开原连接"));
    }
    let mut config = state
        .store
        .get_host_connection(&profile_id)
        .await
        .map_err(storage_error)?;
    // 文件连接只复用传输与认证参数，不继承终端启动命令。
    config.startup = terminal_core::SshStartup::default();
    // SessionManager提供认证与取消生命周期；占位尺寸只作通用元数据，文件工厂不申请PTY。
    let snapshot = state.file_sessions.open(
        &owner,
        request_id.clone(),
        request_id,
        config,
        TerminalSize { cols: 80, rows: 24 },
        Arc::new(FileSink(on_control)),
    )?;
    drop(_gate);
    state.record_host_use(&window, &profile_id).await;
    Ok(snapshot)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyRequest {
    source_session_id: String,
    target_session_id: String,
    source_path: String,
    target_path: String,
    names: Vec<String>,
    transfer_id: String,
}

#[tauri::command]
pub async fn sftp_copy(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: CopyRequest,
    on_progress: Channel<TransferProgress>,
) -> Result<()> {
    let owner = owner(&window)?;
    let CopyRequest {
        source_session_id,
        target_session_id,
        source_path,
        target_path,
        names,
        transfer_id,
    } = request;
    if source_session_id == target_session_id
        || names.is_empty()
        || names.len() > 100
        || uuid::Uuid::parse_str(&transfer_id).is_err()
    {
        return Err(file_error("对拷需要两个独立连接，每批最多100个条目"));
    }
    // 此入口只允许双栏自己持有的独立连接，不借用终端浏览器和分页游标。
    state.file_sessions.get(&owner, &source_session_id)?;
    state.file_sessions.get(&owner, &target_session_id)?;
    let source = browser(&state, &owner, &source_session_id).await?;
    let target = browser(&state, &owner, &target_session_id).await?;
    // try_lock不等待另一端，因此反向请求不会形成循环等待。
    let _source_operation = source
        .operation
        .try_lock()
        .map_err(|_| file_error("源文件浏览器正在操作"))?;
    let _target_operation = target
        .operation
        .try_lock()
        .map_err(|_| file_error("目标文件浏览器正在操作"))?;
    let _budget = state
        .transfers
        .clone()
        .try_acquire_owned()
        .map_err(|_| file_error("传输队列已满"))?;
    let cancel = source.transfer_cancellation();
    let _transfer = TransferRegistration::new(
        &transfer_id,
        cancel.clone(),
        [source.clone(), target.clone()],
    );
    let operation = async {
        // 仅同一文件端点比较真实目录；标题省略端口，共享主机密钥也不能证明同一命名空间。
        if source.identity() == target.identity() {
            let from = source.canonical_path(&source_path).await?;
            let to = target.canonical_path(&target_path).await?;
            for name in &names {
                let root = child_path(&from, name)?;
                let destination = child_path(&to, name)?;
                if destination == root || destination.starts_with(&format!("{root}/")) {
                    return Err(file_error("不能将条目复制到自身或其子目录"));
                }
            }
        }
        let report = |progress| {
            if on_progress.send(progress).is_err() {
                cancel.cancel();
            }
        };
        report(TransferProgress {
            transferred: "0".into(),
            total: None,
            current_name: Some("正在扫描对拷条目…".into()),
            completed_files: Some(0),
            total_files: None,
        });
        terminal_sftp::copy_between(&source, &target, &source_path, &target_path, names, &cancel, |path| {
            let window = window.clone();
            let cancel = cancel.clone();
            async move {
                let (tx, rx) = tokio::sync::oneshot::channel();
                window.dialog().message(format!("目标文件已存在：\n{path}\n\n覆盖将直接截断写入，中断可能保留部分内容。是否覆盖？"))
                    .title("确认对拷覆盖").kind(MessageDialogKind::Warning)
                    .buttons(MessageDialogButtons::OkCancelCustom("覆盖".into(), "保留并停止".into()))
                    .parent(&window).show_with_result(move |answer| {
                        let _ = tx.send(matches!(answer, MessageDialogResult::Custom(label) if label == "覆盖"));
                    });
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => Err(terminal_core::SessionError::cancelled()),
                    answer = rx => answer.map_err(|_| file_error("覆盖确认已关闭")),
                }
            }
        }, report).await?;
        if cancel.is_cancelled() {
            return Err(terminal_core::SessionError::cancelled());
        }
        connected(&state, &owner, &source_session_id)?;
        connected(&state, &owner, &target_session_id)?;
        Ok(())
    };
    // 不在外层丢弃整个传输future，确保copy_file和目录扫描完成句柄关闭。
    operation.await
}

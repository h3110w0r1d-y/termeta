use crate::*;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, mpsc, watch};

struct InputHistory {
    last: u32,
    bytes: usize,
    recent: VecDeque<(u32, Zeroizing<Vec<u8>>)>,
}
struct SessionRecord {
    owner: String,
    events: Arc<SessionEvents>,
    cancel: CancellationToken,
    challenges: Arc<ChallengeBroker>,
    flow: Arc<OutputFlow>,
    input: mpsc::Sender<InputChunk>,
    input_budget: Arc<Semaphore>,
    input_history: tokio::sync::Mutex<InputHistory>,
    resize: watch::Sender<TerminalSize>,
    ended: Mutex<Option<Instant>>,
    close_reason: Mutex<Option<String>>,
}
struct Registry {
    sessions: HashMap<String, Arc<SessionRecord>>,
    requests: HashMap<(String, String), String>,
}
#[derive(Clone)]
pub struct SessionManager {
    factory: Arc<dyn SessionFactory>,
    registry: Arc<Mutex<Registry>>,
    connecting: Arc<Semaphore>,
}
impl SessionManager {
    pub fn new(factory: Arc<dyn SessionFactory>) -> Self {
        Self {
            factory,
            registry: Arc::new(Mutex::new(Registry {
                sessions: HashMap::new(),
                requests: HashMap::new(),
            })),
            connecting: Arc::new(Semaphore::new(4)),
        }
    }
    pub fn capabilities(&self) -> SessionCapabilities {
        self.factory.capabilities()
    }
    pub fn open(
        &self,
        owner: &str,
        request_id: String,
        terminal_id: String,
        config: SshConfig,
        size: TerminalSize,
        sink: Arc<dyn SessionSink>,
    ) -> Result<SessionSnapshot> {
        self.factory.validate(&config)?;
        size.validate()?;
        if uuid::Uuid::parse_str(&request_id).is_err()
            || uuid::Uuid::parse_str(&terminal_id).is_err()
        {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "validation",
                "请求和终端标识必须为 UUID",
            ));
        }
        let mut registry = self.registry.lock().unwrap();
        Self::prune(&mut registry);
        if let Some(id) = registry.requests.get(&(owner.into(), request_id.clone())) {
            return Ok(registry.sessions[id].events.snapshot());
        }
        if registry
            .sessions
            .values()
            .filter(|record| !record.events.snapshot().state.terminal())
            .count()
            >= 20
        {
            return Err(SessionError::new(
                "LIMIT_EXCEEDED",
                "opening",
                "活动会话已达上限",
            ));
        }
        let session_id = uuid::Uuid::new_v4().to_string();
        let snapshot = SessionSnapshot {
            protocol_version: 1,
            request_id: request_id.clone(),
            session_id: session_id.clone(),
            terminal_id,
            kind: self.factory.kind().into(),
            title: format!("{}@{}", config.username, config.address),
            state: SessionState::Created,
            state_revision: 0,
            capabilities: self.factory.capabilities(),
            size,
        };
        let cancel = CancellationToken::new();
        let events = Arc::new(SessionEvents::new(
            snapshot.clone(),
            sink.clone(),
            cancel.clone(),
        ));
        let challenges = Arc::new(ChallengeBroker::new(events.clone(), cancel.clone()));
        let (output, flow, mut forwarder) = output_stream(sink, cancel.clone());
        let (input, input_rx) = mpsc::channel(128);
        let (resize, resize_rx) = watch::channel(size);
        let record = Arc::new(SessionRecord {
            owner: owner.into(),
            events: events.clone(),
            cancel: cancel.clone(),
            challenges: challenges.clone(),
            flow,
            input,
            input_budget: Arc::new(Semaphore::new(65536)),
            input_history: tokio::sync::Mutex::new(InputHistory {
                last: 0,
                bytes: 0,
                recent: VecDeque::new(),
            }),
            resize,
            ended: Mutex::new(None),
            close_reason: Mutex::new(None),
        });
        registry
            .requests
            .insert((owner.into(), request_id), session_id.clone());
        registry.sessions.insert(session_id, record.clone());
        drop(registry);
        let context = DriverContext {
            owner: owner.into(),
            size,
            cancel: cancel.clone(),
            input: input_rx,
            resize: resize_rx,
            output,
            events: events.clone(),
            challenges: challenges.clone(),
        };
        let factory = self.factory.clone();
        let connecting = self.connecting.clone();
        tokio::spawn(async move {
            let result = tokio::select! {
                biased;
                _ = cancel.cancelled() => Err(SessionError::cancelled()),
                permit = connecting.acquire_owned() => match permit {
                    Ok(permit) => {
                        // 连接额度只约束建立阶段，由驱动进入 Connected 后释放。
                        let run = factory.run(config, context);
                        tokio::pin!(run);
                        let mut permit = Some(permit);
                        let mut poll = tokio::time::interval(Duration::from_millis(100));
                        loop { tokio::select! {
                            biased;
                            _ = cancel.cancelled() => break Err(SessionError::cancelled()),
                            result = &mut run => break result,
                            _ = poll.tick(), if permit.is_some() => if events.snapshot().state == SessionState::Connected { permit.take(); },
                        } }
                    }
                    Err(_) => Err(SessionError::cancelled()),
                },
            };
            if result.is_ok() {
                let _ = events.state(SessionState::Closing);
            }
            challenges.revoke();
            let (final_offset, complete) =
                match tokio::time::timeout(Duration::from_secs(3), &mut forwarder).await {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => {
                        cancel.cancel();
                        (record.flow.submitted_offset(), false)
                    }
                    Err(_) => {
                        cancel.cancel();
                        forwarder
                            .await
                            .unwrap_or((record.flow.submitted_offset(), false))
                    }
                };
            let was_connected = events.was_connected();
            let result = match record.flow.take_error() {
                Some(error) => Err(error),
                None => result,
            };
            {
                let mut history = record.input_history.lock().await;
                history.recent.clear();
                history.recent.shrink_to_fit();
                history.bytes = 0;
            }
            {
                // 与显式关闭使用相同锁序，保证终态发布期间迟到关闭不能改写首个原因。
                let close_reason = record.close_reason.lock().unwrap();
                let (state, end, error) = match result {
                    Ok(end) => (SessionState::Closed, end, None),
                    Err(error) if error.code == "CANCELLED" => (
                        SessionState::Closed,
                        SessionEnd::default(),
                        if close_reason.is_some() {
                            None
                        } else {
                            Some(error)
                        },
                    ),
                    Err(error) => (
                        if was_connected {
                            SessionState::Disconnected
                        } else {
                            SessionState::Failed
                        },
                        SessionEnd::default(),
                        Some(error),
                    ),
                };
                // 先汇合输出任务冻结 offset，再发布终态；终态之后无输出 Channel 写入。
                events.finish(
                    state,
                    end,
                    final_offset,
                    complete,
                    error,
                    close_reason.clone(),
                );
            }
            *record.ended.lock().unwrap() = Some(Instant::now());
            // 终态之后只短期保留 ACK 帧端点，十分钟 tombstone 不持有输入或输出历史。
            tokio::time::sleep(Duration::from_secs(3)).await;
            record.flow.retire();
        });
        Ok(snapshot)
    }
    fn prune(registry: &mut Registry) {
        let mut ended: Vec<_> = registry
            .sessions
            .iter()
            .filter_map(|(id, record)| record.ended.lock().unwrap().map(|at| (id.clone(), at)))
            .collect();
        ended.sort_by_key(|(_, at)| *at);
        let excess = ended.len().saturating_sub(1024);
        for (index, (id, at)) in ended.into_iter().enumerate() {
            if index < excess || at.elapsed() > Duration::from_secs(600) {
                registry.sessions.remove(&id);
            }
        }
        registry
            .requests
            .retain(|_, id| registry.sessions.contains_key(id));
    }
    fn owned(&self, owner: &str, id: &str) -> Result<Arc<SessionRecord>> {
        let mut registry = self.registry.lock().unwrap();
        Self::prune(&mut registry);
        let record = registry
            .sessions
            .get(id)
            .ok_or_else(|| SessionError::new("NOT_FOUND", "ownership", "会话不存在或摘要已过期"))?;
        if record.owner != owner {
            return Err(SessionError::new(
                "FORBIDDEN",
                "ownership",
                "无权访问此会话",
            ));
        }
        Ok(record.clone())
    }
    pub fn lookup_request(&self, owner: &str, request_id: &str) -> Option<SessionSnapshot> {
        let mut registry = self.registry.lock().unwrap();
        Self::prune(&mut registry);
        registry
            .requests
            .get(&(owner.to_owned(), request_id.to_owned()))
            .and_then(|id| registry.sessions.get(id))
            .map(|record| record.events.snapshot())
    }
    pub fn cancel_auth(&self, owner: &str, id: &str, challenge: &str) -> Result<()> {
        let record = self.owned(owner, id)?;
        let mut reason = record.close_reason.lock().unwrap();
        record.challenges.cancel_auth(challenge)?;
        reason.get_or_insert_with(|| "authCancelled".into());
        record.cancel.cancel();
        Ok(())
    }
    pub fn get(&self, owner: &str, id: &str) -> Result<SessionSnapshot> {
        Ok(self.owned(owner, id)?.events.snapshot())
    }
    pub fn list(&self, owner: &str) -> Vec<SessionSnapshot> {
        let mut registry = self.registry.lock().unwrap();
        Self::prune(&mut registry);
        registry
            .sessions
            .values()
            .filter(|record| record.owner == owner)
            .map(|record| record.events.snapshot())
            .collect()
    }
    pub fn close(&self, owner: &str, id: &str) -> Result<SessionSnapshot> {
        self.close_with_reason(owner, id, "user")
    }
    pub fn close_with_reason(
        &self,
        owner: &str,
        id: &str,
        reason: &str,
    ) -> Result<SessionSnapshot> {
        if !matches!(
            reason,
            "user"
                | "disconnect"
                | "tabClosed"
                | "windowClosed"
                | "reconnect"
                | "protocolError"
                | "protocolMismatch"
                | "inputFailed"
        ) {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "closing",
                "未知关闭原因",
            ));
        }
        let record = self.owned(owner, id)?;
        Self::request_close(&record, reason)?;
        Ok(record.events.snapshot())
    }
    fn request_close(record: &SessionRecord, reason: &str) -> Result<()> {
        let mut close_reason = record.close_reason.lock().unwrap();
        if record.events.snapshot().state.terminal() {
            return Ok(());
        }
        // 原因只在第一次非终态关闭请求时写入，后续幂等调用不能覆盖用户最初动作。
        close_reason.get_or_insert_with(|| reason.to_owned());
        record.cancel.cancel();
        record.challenges.revoke();
        record.events.state(SessionState::Closing)
    }
    pub fn close_owner(&self, owner: &str) {
        let records: Vec<_> = self
            .registry
            .lock()
            .unwrap()
            .sessions
            .values()
            .filter(|record| record.owner == owner)
            .cloned()
            .collect();
        for record in records {
            let _ = Self::request_close(&record, "windowClosed");
        }
    }
    pub async fn write(&self, owner: &str, id: &str, seq: u32, data: Vec<u8>) -> Result<u32> {
        let record = self.owned(owner, id)?;
        if data.is_empty() || data.len() > 16384 {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "input",
                "每次输入必须为 1–16384 字节",
            ));
        }
        let mut history = record.input_history.lock().await;
        if seq <= history.last {
            return match history.recent.iter().find(|(old, _)| *old == seq) {
                Some((_, previous)) if previous.as_slice() == data.as_slice() => Ok(seq),
                _ => Err(SessionError::new(
                    "PROTOCOL_MISMATCH",
                    "input",
                    "重复输入内容不一致或序号已过期",
                )),
            };
        }
        if history.last.checked_add(1) != Some(seq) {
            return Err(SessionError::new(
                "PROTOCOL_MISMATCH",
                "input",
                "输入序号不连续",
            ));
        }
        if record.events.snapshot().state != SessionState::Connected || record.cancel.is_cancelled()
        {
            return Err(SessionError::new(
                "NOT_CONNECTED",
                "input",
                "会话当前不可输入",
            ));
        }
        let permit = record
            .input_budget
            .clone()
            .try_acquire_many_owned(data.len() as u32)
            .map_err(|_| SessionError::new("LIMIT_EXCEEDED", "input", "输入队列已满，请等待"))?;
        record
            .input
            .try_send(InputChunk {
                data: Bytes::copy_from_slice(&data),
                _permit: permit,
            })
            .map_err(|_| SessionError::new("LIMIT_EXCEEDED", "input", "输入队列不可用"))?;
        history.last = seq;
        history.bytes += data.len();
        history.recent.push_back((seq, Zeroizing::new(data)));
        while history.recent.len() > 128 || history.bytes > 65536 {
            if let Some((_, bytes)) = history.recent.pop_front() {
                history.bytes -= bytes.len();
            }
        }
        Ok(seq)
    }
    pub fn resize(&self, owner: &str, id: &str, size: TerminalSize) -> Result<TerminalSize> {
        size.validate()?;
        let record = self.owned(owner, id)?;
        if record.cancel.is_cancelled() || record.events.snapshot().state.terminal() {
            return Err(SessionError::new("NOT_CONNECTED", "resize", "会话已结束"));
        }
        record.resize.send_replace(size);
        record.events.set_size(size);
        Ok(size)
    }
    pub fn ack(&self, owner: &str, id: &str, offset: u64) -> Result<()> {
        self.owned(owner, id)?.flow.ack(offset)
    }
    pub fn respond_host_key(
        &self,
        owner: &str,
        id: &str,
        challenge: &str,
        decision: HostKeyDecision,
    ) -> Result<()> {
        self.owned(owner, id)?
            .challenges
            .respond_host(challenge, decision)
    }
    pub fn respond_auth(
        &self,
        owner: &str,
        id: &str,
        challenge: &str,
        answers: Vec<String>,
    ) -> Result<()> {
        self.owned(owner, id)?
            .challenges
            .respond_auth(challenge, answers)
    }
}

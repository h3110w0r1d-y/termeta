use crate::*;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{OwnedSemaphorePermit, mpsc, oneshot, watch};

pub struct InputChunk {
    pub data: Bytes,
    pub(crate) _permit: OwnedSemaphorePermit,
}
pub struct DriverContext {
    pub owner: String,
    pub size: TerminalSize,
    pub cancel: CancellationToken,
    pub input: mpsc::Receiver<InputChunk>,
    pub resize: watch::Receiver<TerminalSize>,
    pub output: OutputSender,
    pub events: Arc<SessionEvents>,
    pub challenges: Arc<ChallengeBroker>,
}

pub struct SessionEvents {
    inner: Mutex<(SessionSnapshot, u64)>,
    sink: Mutex<Option<Arc<dyn SessionSink>>>,
    connected: std::sync::atomic::AtomicBool,
    cancel: CancellationToken,
}
impl SessionEvents {
    pub(crate) fn new(
        snapshot: SessionSnapshot,
        sink: Arc<dyn SessionSink>,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            inner: Mutex::new((snapshot, 0)),
            sink: Mutex::new(Some(sink)),
            connected: std::sync::atomic::AtomicBool::new(false),
            cancel,
        }
    }
    pub fn snapshot(&self) -> SessionSnapshot {
        self.inner.lock().unwrap().0.clone()
    }
    pub(crate) fn was_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Acquire)
    }
    pub fn state(&self, state: SessionState) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        // 关闭一旦开始，迟到的握手回调不能把状态恢复为连接中。
        if inner.0.state == state
            || inner.0.state.terminal()
            || (inner.0.state == SessionState::Closing && state != SessionState::Closed)
        {
            return Ok(());
        }
        inner.0.state = state;
        if state == SessionState::Connected {
            self.connected
                .store(true, std::sync::atomic::Ordering::Release);
        }
        inner.0.state_revision += 1;
        let payload = EventPayload::StateChanged {
            snapshot: inner.0.clone(),
        };
        self.emit_locked(&mut inner, payload)
    }
    pub(crate) fn set_size(&self, size: TerminalSize) {
        self.inner.lock().unwrap().0.size = size;
    }
    pub fn emit(&self, payload: EventPayload) -> Result<()> {
        self.emit_locked(&mut self.inner.lock().unwrap(), payload)
    }
    fn emit_locked(&self, inner: &mut (SessionSnapshot, u64), payload: EventPayload) -> Result<()> {
        inner.1 += 1;
        let event = SessionEvent {
            protocol_version: 1,
            request_id: inner.0.request_id.clone(),
            session_id: inner.0.session_id.clone(),
            event_seq: inner.1,
            payload,
        };
        let sink = self.sink.lock().unwrap().clone();
        let result = sink.map_or(Ok(()), |sink| sink.event(event));
        if result.is_err() {
            self.cancel.cancel();
        }
        result
    }
    pub(crate) fn finish(
        &self,
        state: SessionState,
        end: SessionEnd,
        offset: u64,
        complete: bool,
        error: Option<SessionError>,
        close_reason: Option<String>,
    ) {
        let mut inner = self.inner.lock().unwrap();
        inner.0.state = state;
        inner.0.state_revision += 1;
        let _ = self.emit_locked(
            &mut inner,
            EventPayload::Ended {
                final_state: state,
                reason: if state == SessionState::Closed {
                    close_reason.unwrap_or_else(|| {
                        if error.is_some() {
                            "cancelled"
                        } else {
                            "remoteExit"
                        }
                        .into()
                    })
                } else if error
                    .as_ref()
                    .is_some_and(|error| error.code == "CONSUMER_STALLED")
                {
                    "consumerStalled".into()
                } else if state == SessionState::Disconnected {
                    "connectionLost".into()
                } else {
                    "failed".into()
                },
                exit_code: end.exit_code,
                exit_signal: end.exit_signal,
                final_offset: offset.to_string(),
                output_complete: complete,
                error,
            },
        );
        self.sink.lock().unwrap().take();
    }
}

enum ChallengeReply {
    Host(HostKeyDecision),
    Auth(Vec<Zeroizing<String>>),
}
struct PendingChallenge {
    id: String,
    answers: usize,
    host: bool,
    expires: std::time::Instant,
    sender: oneshot::Sender<ChallengeReply>,
}
pub struct ChallengeBroker {
    pending: Mutex<Option<PendingChallenge>>,
    events: Arc<SessionEvents>,
    cancel: CancellationToken,
}
impl ChallengeBroker {
    pub(crate) fn new(events: Arc<SessionEvents>, cancel: CancellationToken) -> Self {
        Self {
            pending: Mutex::new(None),
            events,
            cancel,
        }
    }
    pub fn revoke(&self) {
        self.pending.lock().unwrap().take();
    }
    pub fn cancel_auth(&self, id: &str) -> Result<()> {
        let mut pending = self.pending.lock().unwrap();
        if !pending.as_ref().is_some_and(|challenge| {
            challenge.id == id && !challenge.host && challenge.expires > std::time::Instant::now()
        }) {
            return Err(SessionError::new(
                "INVALID_CHALLENGE",
                "authentication",
                "认证挑战已失效",
            ));
        }
        pending.take();
        self.cancel.cancel();
        Ok(())
    }
    pub fn respond_host(&self, id: &str, decision: HostKeyDecision) -> Result<()> {
        self.respond(id, ChallengeReply::Host(decision))
    }
    pub fn respond_auth(&self, id: &str, answers: Vec<String>) -> Result<()> {
        if answers.len() > 16 || answers.iter().map(String::len).sum::<usize>() > 16384 {
            return Err(SessionError::new(
                "INVALID_CONFIG",
                "authentication",
                "认证响应超过限制",
            ));
        }
        self.respond(
            id,
            ChallengeReply::Auth(answers.into_iter().map(Zeroizing::new).collect()),
        )
    }
    fn respond(&self, id: &str, reply: ChallengeReply) -> Result<()> {
        let mut guard = self.pending.lock().unwrap();
        let valid = guard.as_ref().is_some_and(|p| {
            p.id == id
                && p.expires > std::time::Instant::now()
                && match &reply {
                    ChallengeReply::Host(_) => p.host,
                    ChallengeReply::Auth(a) => !p.host && p.answers == a.len(),
                }
        });
        if !valid || self.cancel.is_cancelled() {
            return Err(SessionError::new(
                "INVALID_CHALLENGE",
                "authentication",
                "挑战已失效或响应不匹配",
            ));
        }
        guard
            .take()
            .unwrap()
            .sender
            .send(reply)
            .map_err(|_| SessionError::new("INVALID_CHALLENGE", "authentication", "挑战已过期"))
    }
    async fn request(
        &self,
        host: bool,
        answers: usize,
        make: impl FnOnce(String, u64) -> EventPayload,
    ) -> Result<ChallengeReply> {
        let id = uuid::Uuid::new_v4().to_string();
        let (sender, receiver) = oneshot::channel();
        *self.pending.lock().unwrap() = Some(PendingChallenge {
            id: id.clone(),
            host,
            answers,
            expires: std::time::Instant::now() + Duration::from_secs(120),
            sender,
        });
        let deadline = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            + 120_000;
        if let Err(error) = self.events.emit(make(id.clone(), deadline)) {
            self.revoke();
            return Err(error);
        }
        let result = tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(SessionError::cancelled()),
            result = tokio::time::timeout(Duration::from_secs(120), receiver) => match result {
                Ok(Ok(reply)) => Ok(reply),
                _ => Err(SessionError::new("AUTH_TIMEOUT", "authentication", "等待确认已超时")),
            },
        };
        let mut pending = self.pending.lock().unwrap();
        if pending.as_ref().is_some_and(|p| p.id == id) {
            pending.take();
        }
        result
    }
    pub async fn host_key(&self, key: &HostKey) -> Result<HostKeyDecision> {
        self.events.state(SessionState::AwaitingHostKey)?;
        match self
            .request(true, 0, |challenge_id, expires_at| {
                EventPayload::HostKeyChallenge {
                    challenge_id,
                    endpoint: key.endpoint.clone(),
                    algorithm: key.algorithm.clone(),
                    fingerprint: key.fingerprint.clone(),
                    expires_at,
                }
            })
            .await?
        {
            ChallengeReply::Host(decision) => Ok(decision),
            _ => unreachable!(),
        }
    }
    pub async fn auth(
        &self,
        name: String,
        instructions: String,
        prompts: Vec<AuthPrompt>,
    ) -> Result<Vec<Zeroizing<String>>> {
        validate_auth_challenge(&name, &instructions, &prompts)?;
        self.events.state(SessionState::AwaitingAuth)?;
        let reply = self
            .request(false, prompts.len(), |challenge_id, expires_at| {
                EventPayload::AuthChallenge {
                    challenge_id,
                    name,
                    instructions,
                    prompts,
                    expires_at,
                }
            })
            .await?;
        self.events.state(SessionState::Authenticating)?;
        match reply {
            ChallengeReply::Auth(answers) => Ok(answers),
            _ => unreachable!(),
        }
    }
}

/// 自动响应与用户交互共用提示限额，不能因跳过弹窗而绕过校验。
pub fn validate_auth_challenge(
    name: &str,
    instructions: &str,
    prompts: &[AuthPrompt],
) -> Result<()> {
    if name.len() > 1024
        || instructions.len() > 8192
        || prompts.len() > 16
        || prompts.iter().map(|p| p.text.len()).sum::<usize>() > 16384
    {
        return Err(SessionError::new(
            "LIMIT_EXCEEDED",
            "authentication",
            "服务端认证提示超过限制",
        ));
    }
    Ok(())
}

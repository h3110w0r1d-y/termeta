//! 生命周期与流协议的独立领域测试；不建立网络连接，也不依赖桌面壳。
use crate::*;
use async_trait::async_trait;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Notify;

#[derive(Default)]
struct Capture {
    events: Mutex<Vec<SessionEvent>>,
    frames: Mutex<Vec<Vec<u8>>>,
    changed: Notify,
}
impl SessionSink for Capture {
    fn output(&self, frame: Vec<u8>) -> Result<()> {
        self.frames.lock().unwrap().push(frame);
        self.changed.notify_one();
        Ok(())
    }
    fn event(&self, event: SessionEvent) -> Result<()> {
        self.events.lock().unwrap().push(event);
        self.changed.notify_one();
        Ok(())
    }
}
impl Capture {
    async fn wait_for(&self, ready: impl Fn() -> bool) {
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if ready() {
                    return;
                }
                self.changed.notified().await;
            }
        })
        .await
        .unwrap();
    }
}
struct FakeFactory {
    attempts: AtomicUsize,
    trust: bool,
}
#[async_trait]
impl SessionFactory for FakeFactory {
    fn kind(&self) -> &'static str {
        "ssh"
    }
    fn capabilities(&self) -> SessionCapabilities {
        SessionCapabilities::ssh()
    }
    fn validate(&self, config: &SessionConfig) -> Result<()> {
        config.validate()
    }
    async fn run(&self, _config: SessionConfig, context: DriverContext) -> Result<SessionEnd> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        if self.trust {
            let decision = context
                .challenges
                .host_key(&HostKey {
                    endpoint: HostEndpoint {
                        host: "example.invalid".into(),
                        canonical_host: "example.invalid".into(),
                        port: 22,
                    },
                    algorithm: "ssh-ed25519".into(),
                    public_key: vec![1],
                    fingerprint: "SHA256:test".into(),
                })
                .await?;
            if matches!(decision, HostKeyDecision::Reject) {
                return Err(SessionError::cancelled());
            }
        }
        context.events.state(SessionState::Connected)?;
        context
            .output
            .send(Bytes::from_static(b"\xf0\x9f\x98\x80\0\xff"))
            .await?;
        context.cancel.cancelled().await;
        Err(SessionError::cancelled())
    }
}
fn config() -> SshConfig {
    SshConfig {
        startup: SshStartup::default(),
        config_version: 1,
        system_profile_id: None,
        address: "example.invalid".into(),
        port: 22,
        username: "user".into(),
        auth_plan: vec![AuthStep::Password {
            credential_id: None,
        }],
    }
}
fn size() -> TerminalSize {
    TerminalSize { cols: 80, rows: 24 }
}
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[test]
fn ssh_commands_validate_bytes_without_rewriting_the_command() {
    let mut ssh = config();
    assert!(!ssh.startup.is_exec());
    for command in [
        "",
        " \n\t",
        "btop\0ignored",
        &"a".repeat(8193),
        &"中".repeat(2731),
    ] {
        ssh.startup = SshStartup::Exec {
            command: command.into(),
        };
        assert_eq!(ssh.validate().unwrap_err().code, "INVALID_CONFIG");
    }
    let command = "  TERM=xterm-256color /usr/bin/btop --utf-force\n";
    ssh.startup = SshStartup::Exec {
        command: command.into(),
    };
    ssh.validate().unwrap();
    assert_eq!(
        ssh.startup,
        SshStartup::Exec {
            command: command.into()
        }
    );
    ssh.startup = SshStartup::Exec {
        command: "a".repeat(8192),
    };
    ssh.validate().unwrap();
}

#[tokio::test]
async fn command_mode_is_part_of_the_actual_session_snapshot() {
    let manager = SessionManager::new(Arc::new(FakeFactory {
        attempts: AtomicUsize::new(0),
        trust: false,
    }));
    for command in [false, true] {
        let mut ssh = config();
        if command {
            ssh.startup = SshStartup::Exec {
                command: "btop".into(),
            };
        }
        let snapshot = manager
            .open(
                "main",
                id(),
                id(),
                ssh,
                size(),
                Arc::new(Capture::default()),
            )
            .unwrap();
        assert_eq!(snapshot.ssh_command, command);
        assert_eq!(
            manager
                .get("main", &snapshot.session_id)
                .unwrap()
                .ssh_command,
            command
        );
        manager.close("main", &snapshot.session_id).unwrap();
    }
}

#[tokio::test]
async fn requests_are_idempotent_and_owner_is_enforced() {
    let factory = Arc::new(FakeFactory {
        attempts: AtomicUsize::new(0),
        trust: false,
    });
    let manager = SessionManager::new(factory.clone());
    let capture = Arc::new(Capture::default());
    let request = id();
    let first = manager
        .open(
            "main",
            request.clone(),
            id(),
            config(),
            size(),
            capture.clone(),
        )
        .unwrap();
    let second = manager
        .open(
            "main",
            request,
            id(),
            config(),
            size(),
            Arc::new(Capture::default()),
        )
        .unwrap();
    assert_eq!(first.session_id, second.session_id);
    capture
        .wait_for(|| !capture.frames.lock().unwrap().is_empty())
        .await;
    assert_eq!(factory.attempts.load(Ordering::SeqCst), 1);
    assert_eq!(
        manager.close("other", &first.session_id).unwrap_err().code,
        "FORBIDDEN"
    );
    manager
        .close_with_reason("main", &first.session_id, "disconnect")
        .unwrap();
    manager
        .close_with_reason("main", &first.session_id, "tabClosed")
        .unwrap();
    capture
        .wait_for(|| {
            manager
                .get("main", &first.session_id)
                .unwrap()
                .state
                .terminal()
        })
        .await;
    let ended = manager.close("main", &first.session_id).unwrap();
    assert_eq!(ended.state, SessionState::Closed);
    let events = capture.events.lock().unwrap();
    let reason = events.iter().find_map(|event| match &event.payload {
        EventPayload::Ended { reason, .. } => Some(reason.as_str()),
        _ => None,
    });
    assert_eq!(reason, Some("disconnect"));
}

#[tokio::test]
async fn binary_frames_and_input_deduplication_preserve_bytes() {
    let manager = SessionManager::new(Arc::new(FakeFactory {
        attempts: AtomicUsize::new(0),
        trust: false,
    }));
    let capture = Arc::new(Capture::default());
    let session = manager
        .open("main", id(), id(), config(), size(), capture.clone())
        .unwrap();
    capture
        .wait_for(|| !capture.frames.lock().unwrap().is_empty())
        .await;
    let frame = capture.frames.lock().unwrap()[0].clone();
    assert_eq!(frame[0], 1);
    assert_eq!(u32::from_le_bytes(frame[4..8].try_into().unwrap()), 1);
    assert_eq!(u64::from_le_bytes(frame[8..16].try_into().unwrap()), 6);
    assert_eq!(&frame[24..], b"\xf0\x9f\x98\x80\0\xff");
    assert_eq!(
        manager
            .ack("main", &session.session_id, 5)
            .unwrap_err()
            .code,
        "PROTOCOL_MISMATCH"
    );
    manager.ack("main", &session.session_id, 6).unwrap();
    assert_eq!(
        manager
            .write("main", &session.session_id, 1, vec![0, 255])
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        manager
            .write("main", &session.session_id, 1, vec![0, 255])
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        manager
            .write("main", &session.session_id, 1, vec![1])
            .await
            .unwrap_err()
            .code,
        "PROTOCOL_MISMATCH"
    );
    assert_eq!(
        manager
            .write("main", &session.session_id, 3, vec![1])
            .await
            .unwrap_err()
            .code,
        "PROTOCOL_MISMATCH"
    );
    manager.close("main", &session.session_id).unwrap();
}

#[tokio::test]
async fn host_challenge_is_bound_to_session_and_single_use() {
    let manager = SessionManager::new(Arc::new(FakeFactory {
        attempts: AtomicUsize::new(0),
        trust: true,
    }));
    let capture = Arc::new(Capture::default());
    let session = manager
        .open("main", id(), id(), config(), size(), capture.clone())
        .unwrap();
    capture
        .wait_for(|| {
            capture
                .events
                .lock()
                .unwrap()
                .iter()
                .any(|event| matches!(event.payload, EventPayload::HostKeyChallenge { .. }))
        })
        .await;
    let challenge = capture
        .events
        .lock()
        .unwrap()
        .iter()
        .find_map(|event| match &event.payload {
            EventPayload::HostKeyChallenge { challenge_id, .. } => Some(challenge_id.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        manager
            .respond_host_key(
                "other",
                &session.session_id,
                &challenge,
                HostKeyDecision::AllowOnce
            )
            .unwrap_err()
            .code,
        "FORBIDDEN"
    );
    assert_eq!(
        manager
            .respond_host_key(
                "main",
                &session.session_id,
                "stale",
                HostKeyDecision::AllowOnce
            )
            .unwrap_err()
            .code,
        "INVALID_CHALLENGE"
    );
    manager
        .respond_host_key(
            "main",
            &session.session_id,
            &challenge,
            HostKeyDecision::AllowOnce,
        )
        .unwrap();
    assert_eq!(
        manager
            .respond_host_key(
                "main",
                &session.session_id,
                &challenge,
                HostKeyDecision::AllowOnce
            )
            .unwrap_err()
            .code,
        "INVALID_CHALLENGE"
    );
    capture
        .wait_for(|| !capture.frames.lock().unwrap().is_empty())
        .await;
    manager.close("main", &session.session_id).unwrap();
}

struct FakeLocalFactory;
#[async_trait]
impl SessionFactory for FakeLocalFactory {
    fn kind(&self) -> &'static str {
        "local"
    }
    fn capabilities(&self) -> SessionCapabilities {
        SessionCapabilities::ssh()
    }
    fn validate(&self, config: &SessionConfig) -> Result<()> {
        assert!(matches!(config, SessionConfig::Local));
        Ok(())
    }
    async fn run(&self, _: SessionConfig, context: DriverContext) -> Result<SessionEnd> {
        context.events.state(SessionState::Connected)?;
        context.cancel.cancelled().await;
        Err(SessionError::cancelled())
    }
}

struct FakeSerialFactory;
#[async_trait]
impl SessionFactory for FakeSerialFactory {
    fn kind(&self) -> &'static str {
        "serial"
    }
    fn capabilities(&self) -> SessionCapabilities {
        SessionCapabilities {
            resize: false,
            pty: false,
            binary_output: true,
            serial_control: false,
        }
    }
    fn validate(&self, config: &SessionConfig) -> Result<()> {
        config.validate()
    }
    async fn run(&self, _: SessionConfig, context: DriverContext) -> Result<SessionEnd> {
        context.events.state(SessionState::Connected)?;
        context.cancel.cancelled().await;
        Err(SessionError::cancelled())
    }
}

#[tokio::test]
async fn serial_uses_owner_boundary_and_rejects_remote_resize() {
    let manager = SessionManager::new(Arc::new(FakeSerialFactory));
    let capture = Arc::new(Capture::default());
    let snapshot = manager
        .open(
            "main",
            id(),
            id(),
            SessionConfig::Serial(SerialConfig {
                path: "COM3".into(),
                baud_rate: 9600,
                charset: "UTF-8".into(),
                data_bits: 8,
                stop_bits: 1,
                flow_control: SerialFlowControl::None,
                parity: SerialParity::None,
            }),
            size(),
            capture.clone(),
        )
        .unwrap();
    capture
        .wait_for(|| {
            manager.get("main", &snapshot.session_id).unwrap().state == SessionState::Connected
        })
        .await;
    assert_eq!(snapshot.kind, "serial");
    assert!(!snapshot.capabilities.resize);
    assert_eq!(
        manager
            .resize("main", &snapshot.session_id, size())
            .unwrap_err()
            .code,
        "UNSUPPORTED_OPERATION"
    );
    assert_eq!(
        manager
            .resize("other", &snapshot.session_id, size())
            .unwrap_err()
            .code,
        "FORBIDDEN"
    );
    manager.close_owner("main");
    capture
        .wait_for(|| {
            manager
                .get("main", &snapshot.session_id)
                .unwrap()
                .state
                .terminal()
        })
        .await;
}

#[tokio::test]
async fn local_factory_uses_shared_registry_and_owner_boundary() {
    let ssh = Arc::new(FakeFactory {
        attempts: AtomicUsize::new(0),
        trust: false,
    });
    let manager = SessionManager::new(ssh.clone()).with_factory(Arc::new(FakeLocalFactory));
    let capture = Arc::new(Capture::default());
    let request = uuid::Uuid::new_v4().to_string();
    let terminal = uuid::Uuid::new_v4().to_string();
    let snapshot = manager
        .open(
            "main",
            request.clone(),
            terminal.clone(),
            SessionConfig::Local,
            TerminalSize { cols: 80, rows: 24 },
            capture.clone(),
        )
        .unwrap();
    assert_eq!(snapshot.kind, "local");
    assert_eq!(snapshot.title, "本地终端");
    assert_eq!(manager.available_kinds().len(), 2);
    assert_eq!(
        manager.get("other", &snapshot.session_id).unwrap_err().code,
        "FORBIDDEN"
    );
    let duplicate = manager
        .open(
            "main",
            request,
            terminal,
            SessionConfig::Local,
            TerminalSize { cols: 80, rows: 24 },
            capture.clone(),
        )
        .unwrap();
    assert_eq!(snapshot.session_id, duplicate.session_id);
    capture
        .wait_for(|| {
            manager.get("main", &snapshot.session_id).unwrap().state == SessionState::Connected
        })
        .await;
    assert_eq!(ssh.attempts.load(Ordering::SeqCst), 0);
    manager.close_owner("main");
    capture
        .wait_for(|| {
            manager
                .get("main", &snapshot.session_id)
                .unwrap()
                .state
                .terminal()
        })
        .await;
}

struct EndWithoutClientClose;
#[async_trait]
impl SessionFactory for EndWithoutClientClose {
    fn kind(&self) -> &'static str {
        "ssh"
    }
    fn capabilities(&self) -> SessionCapabilities {
        SessionCapabilities::ssh()
    }
    fn validate(&self, config: &SessionConfig) -> Result<()> {
        config.validate()
    }
    async fn run(&self, _config: SessionConfig, context: DriverContext) -> Result<SessionEnd> {
        context.events.state(SessionState::Connected)?;
        Ok(SessionEnd::default())
    }
}

#[tokio::test]
async fn remote_end_revokes_resources_without_frontend_close() {
    let capture = Arc::new(Capture::default());
    let manager = SessionManager::new(Arc::new(EndWithoutClientClose));
    let observer = Arc::new(Mutex::new(std::sync::Weak::<SessionManager>::new()));
    let query = observer.clone();
    let notifications = Arc::new(AtomicUsize::new(0));
    let count = notifications.clone();
    let changed = capture.clone();
    let manager = Arc::new(manager.with_resource_cleanup(Arc::new(move |owner, id| {
        // 回调内重新查询同一 manager，验证撤销入口不占用全局会话表锁。
        assert!(
            query
                .lock()
                .unwrap()
                .upgrade()
                .unwrap()
                .get(owner, id)
                .is_ok()
        );
        count.fetch_add(1, Ordering::SeqCst);
        changed.changed.notify_one();
    })));
    *observer.lock().unwrap() = Arc::downgrade(&manager);
    let snapshot = manager
        .open("main", id(), id(), config(), size(), capture.clone())
        .unwrap();
    let resources = manager
        .resource_cancellation("main", &snapshot.session_id)
        .unwrap();
    capture
        .wait_for(|| notifications.load(Ordering::SeqCst) == 1)
        .await;
    assert!(resources.is_cancelled());
    manager.close("main", &snapshot.session_id).unwrap();
    manager.close_owner("main");
    assert_eq!(notifications.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn close_and_owner_destruction_revoke_resources_immediately_once() {
    let notifications = Arc::new(AtomicUsize::new(0));
    let count = notifications.clone();
    let manager = SessionManager::new(Arc::new(FakeFactory {
        attempts: AtomicUsize::new(0),
        trust: false,
    }))
    .with_resource_cleanup(Arc::new(move |_, _| {
        count.fetch_add(1, Ordering::SeqCst);
    }));
    for by_owner in [false, true] {
        let snapshot = manager
            .open(
                "main",
                id(),
                id(),
                config(),
                size(),
                Arc::new(Capture::default()),
            )
            .unwrap();
        let resources = manager
            .resource_cancellation("main", &snapshot.session_id)
            .unwrap();
        assert!(!resources.is_cancelled());
        if by_owner {
            manager.close_owner("main");
        } else {
            manager.close("main", &snapshot.session_id).unwrap();
        }
        assert!(resources.is_cancelled());
        // 撤销后才登记的任务必须继承取消，不得留在选择器/文件承诺等待中。
        assert!(resources.child_token().is_cancelled());
        manager.close("main", &snapshot.session_id).unwrap();
    }
    assert_eq!(notifications.load(Ordering::SeqCst), 2);
}

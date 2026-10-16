//! 需用户授权后执行；仅使用本机临时端口，不连接外部主机。
use super::*;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

#[derive(Default)]
struct Capture {
    bytes: Mutex<Vec<u8>>,
}
impl SessionSink for Capture {
    fn output(&self, frame: Vec<u8>) -> Result<()> {
        self.bytes.lock().unwrap().extend_from_slice(&frame[24..]);
        Ok(())
    }
    fn event(&self, _: SessionEvent) -> Result<()> {
        Ok(())
    }
}
async fn wait(ready: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !ready() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
fn open(manager: &SessionManager, port: u16, capture: Arc<Capture>) -> SessionSnapshot {
    // 固定且每个 manager 内唯一的有效 UUID；不新增随机依赖。
    manager
        .open(
            "main",
            "00000000-0000-4000-8000-000000000001".into(),
            "00000000-0000-4000-8000-000000000002".into(),
            SessionConfig::Telnet(TelnetConfig {
                config_version: 1,
                address: "127.0.0.1".into(),
                port,
            }),
            TerminalSize { cols: 80, rows: 24 },
            capture,
        )
        .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn fragmented_negotiation_input_resize_and_remote_eof_use_shared_session_flow() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        for byte in b"\xff\xfe\0\xff\xfc\0\xff\xfb\x01\xff\xfd\x18\xff\xfd\x1flogin:\r\0\n" {
            socket.write_all(&[*byte]).await.unwrap();
        }
        let mut wire = Vec::new();
        let mut buffer = [0; 1024];
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let count = socket.read(&mut buffer).await.unwrap();
                assert_ne!(count, 0);
                wire.extend_from_slice(&buffer[..count]);
                if wire.windows(6).any(|part| part == b"user\r\n")
                    && wire
                        .windows(9)
                        .any(|part| part == b"\xff\xfa\x1f\0\x64\0\x28\xff\xf0")
                {
                    break;
                }
            }
        })
        .await
        .unwrap();
        socket.write_all(b"ready\r\n").await.unwrap();
        wire
    });
    let manager = SessionManager::new(Arc::new(TelnetFactory));
    let capture = Arc::new(Capture::default());
    let snapshot = open(&manager, port, capture.clone());
    assert_eq!(snapshot.kind, "telnet");
    assert!(snapshot.capabilities.resize);
    assert!(!snapshot.capabilities.pty);
    wait(|| capture.bytes.lock().unwrap().ends_with(b"login:\r\n")).await;
    assert!(
        manager
            .write("other", &snapshot.session_id, 1, b"bad".to_vec())
            .await
            .is_err()
    );
    manager
        .write("main", &snapshot.session_id, 1, b"user\r".to_vec())
        .await
        .unwrap();
    manager
        .resize(
            "main",
            &snapshot.session_id,
            TerminalSize {
                cols: 100,
                rows: 40,
            },
        )
        .unwrap();
    let wire = server.await.unwrap();
    assert!(wire.windows(3).any(|part| part == b"\xff\xfd\x01"));
    wait(|| {
        manager
            .get("main", &snapshot.session_id)
            .unwrap()
            .state
            .terminal()
    })
    .await;
    assert_eq!(*capture.bytes.lock().unwrap(), b"login:\r\nready\r\n");
    assert_eq!(
        manager.get("main", &snapshot.session_id).unwrap().state,
        SessionState::Closed
    );
}

#[tokio::test(flavor = "current_thread")]
async fn cancellation_finishes_when_output_has_no_acknowledgements() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let data = [b'x'; 8192];
        while socket.write_all(&data).await.is_ok() {}
    });
    let manager = SessionManager::new(Arc::new(TelnetFactory));
    let capture = Arc::new(Capture::default());
    let snapshot = open(&manager, port, capture.clone());
    wait(|| capture.bytes.lock().unwrap().len() >= 248 * 1024).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    manager.close("main", &snapshot.session_id).unwrap();
    wait(|| {
        manager
            .get("main", &snapshot.session_id)
            .unwrap()
            .state
            .terminal()
    })
    .await;
    assert_eq!(
        manager.get("main", &snapshot.session_id).unwrap().state,
        SessionState::Closed
    );
    assert!(capture.bytes.lock().unwrap().len() <= 256 * 1024);
    server.abort();
}

use crate::{host_key::HostKeyHandler, ssh_error};
use std::{
    borrow::Cow,
    net::Shutdown,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use terminal_core::*;
use tokio::net::{TcpStream, lookup_host};

/// 即使 russh 内部任务仍持有 socket，取消或丢弃驱动也会关闭所有复制句柄对应的传输。
pub(crate) struct TransportGuard(std::net::TcpStream);
impl Drop for TransportGuard {
    fn drop(&mut self) {
        let _ = self.0.shutdown(Shutdown::Both);
    }
}
pub(crate) async fn connect(
    config: &SshConfig,
    context: &DriverContext,
    trust: Arc<dyn HostTrustStore>,
) -> Result<(russh::client::Handle<HostKeyHandler>, TransportGuard)> {
    let addresses: Vec<_> = tokio::time::timeout(
        Duration::from_secs(10),
        lookup_host((config.address.as_str(), config.port)),
    )
    .await
    .map_err(|_| ssh_error("DNS_FAILED", "dns", "域名解析超时"))?
    .map_err(|_| ssh_error("DNS_FAILED", "dns", "无法解析主机地址"))?
    .collect();
    let socket = tokio::time::timeout(Duration::from_secs(15), async {
        for address in addresses.into_iter().take(16) {
            if let Ok(socket) =
                tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(address)).await
            {
                if let Ok(socket) = socket {
                    return Ok(socket);
                }
            }
        }
        Err(ssh_error("IO_FAILED", "tcp", "无法连接主机地址与端口"))
    })
    .await
    .map_err(|_| ssh_error("CONNECT_TIMEOUT", "tcp", "TCP 连接超时"))??;
    socket
        .set_nodelay(true)
        .map_err(|_| ssh_error("IO_FAILED", "tcp", "无法配置 TCP 连接"))?;
    let socket = socket
        .into_std()
        .map_err(|_| ssh_error("IO_FAILED", "tcp", "无法配置传输句柄"))?;
    let transport = TransportGuard(
        socket
            .try_clone()
            .map_err(|_| ssh_error("IO_FAILED", "tcp", "无法配置传输取消句柄"))?,
    );
    let socket = TcpStream::from_std(socket)
        .map_err(|_| ssh_error("IO_FAILED", "tcp", "无法注册 TCP 连接"))?;
    let waiting = Arc::new(AtomicBool::new(false));
    let failure = Arc::new(std::sync::Mutex::new(None));
    let handler = HostKeyHandler {
        endpoint: config.endpoint(),
        trust,
        challenges: context.challenges.clone(),
        waiting: waiting.clone(),
        failure: failure.clone(),
    };
    let mut options = russh::client::Config {
        window_size: 262144,
        maximum_packet_size: 32768,
        channel_buffer_size: 16,
        keepalive_interval: Some(Duration::from_secs(30)),
        keepalive_max: 3,
        inactivity_timeout: None,
        nodelay: true,
        ..Default::default()
    };
    // release 的默认 host key 列表含 ssh-rsa/SHA-1，必须显式移除而非声称默认全部安全。
    options.preferred.key = Cow::Owned(
        options
            .preferred
            .key
            .iter()
            .filter(|key| !matches!(key, russh::keys::Algorithm::Rsa { hash: None }))
            .cloned()
            .collect(),
    );
    let handshake = russh::client::connect_stream(Arc::new(options), socket, handler);
    tokio::pin!(handshake);
    let network_deadline = async {
        let mut elapsed = Duration::ZERO;
        let mut last = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let now = Instant::now();
            if !waiting.load(Ordering::Acquire) {
                elapsed += now.duration_since(last);
            }
            last = now;
            if elapsed >= Duration::from_secs(20) {
                break;
            }
        }
    };
    let handle = tokio::select! {
        biased;
        _ = context.cancel.cancelled() => return Err(SessionError::cancelled()),
        _ = network_deadline => return Err(ssh_error("CONNECT_TIMEOUT", "handshake", "SSH 握手超时")),
        result = &mut handshake => match result {
            Ok(handle) => handle,
            Err(_) => return Err(failure.lock().unwrap().take().unwrap_or_else(|| ssh_error("HANDSHAKE_FAILED", "handshake", "SSH 握手失败"))),
        }
    };
    Ok((handle, transport))
}

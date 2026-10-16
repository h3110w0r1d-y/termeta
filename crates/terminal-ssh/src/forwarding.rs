use crate::{host_key::HostKeyHandler, network, socks, ssh_error};
use russh::{Channel, client};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use terminal_core::*;
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
    sync::mpsc,
    task::JoinSet,
};

pub(crate) struct Incoming {
    pub channel: Channel<client::Msg>,
    pub reply: client::ChannelOpenHandle,
}
#[derive(Clone)]
pub(crate) struct RemoteForward {
    pub rule: ForwardRule,
    pub incoming: mpsc::Sender<Incoming>,
}
impl RemoteForward {
    pub fn matches(&self, address: &str, port: u32) -> bool {
        if port != u32::from(self.rule.bind_port) {
            return false;
        }
        let bind: IpAddr = self
            .rule
            .bind_address
            .parse()
            .expect("validated bind address");
        address == self.rule.bind_address
            || address.parse::<IpAddr>().is_ok_and(|ip| ip == bind)
            || bind.is_unspecified()
            || (bind.is_loopback() && address == "localhost")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_channels_must_match_registered_listener() {
        let (incoming, _) = mpsc::channel(1);
        let forward = RemoteForward {
            incoming,
            rule: ForwardRule {
                mode: ForwardMode::Remote,
                bind_address: "::1".into(),
                bind_port: 8080,
                target_address: Some("127.0.0.1".into()),
                target_port: Some(80),
            },
        };
        assert!(forward.matches("0:0:0:0:0:0:0:1", 8080));
        assert!(!forward.matches("::1", 8081));
        assert!(!forward.matches("2001:db8::1", 8080));
    }
}

pub(crate) async fn run(
    connection: client::Handle<HostKeyHandler>,
    rule: ForwardRule,
    mut incoming: mpsc::Receiver<Incoming>,
    context: DriverContext,
) -> Result<SessionEnd> {
    let connection = Arc::new(connection);
    let listener = if rule.mode == ForwardMode::Remote {
        network(
            "forwarding",
            connection.tcpip_forward(rule.bind_address.clone(), u32::from(rule.bind_port)),
        )
        .await
        .map_err(|_| {
            ssh_error(
                "FORWARD_DENIED",
                "forwarding",
                "服务器拒绝远程监听，请检查端口占用与 AllowTcpForwarding/GatewayPorts 设置",
            )
        })?;
        None
    } else {
        Some(
            TcpListener::bind(SocketAddr::new(
                rule.bind_address.parse().unwrap(),
                rule.bind_port,
            ))
            .await
            .map_err(|_| {
                ssh_error(
                    "BIND_FAILED",
                    "forwarding",
                    "无法监听本机地址与端口，请检查地址、权限和端口占用",
                )
            })?,
        )
    };
    // 监听真正就绪后才进入 Connected，卡片才能高亮。
    context.events.state(SessionState::Connected)?;
    let mut tasks = JoinSet::new();
    let mut health = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            biased;
            _ = context.cancel.cancelled() => return Err(SessionError::cancelled()),
            _ = health.tick() => if connection.is_closed() {
                return Err(ssh_error("IO_FAILED", "forwarding", "SSH 转发连接已断开，请重新启动"));
            },
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {},
            accepted = async { listener.as_ref().unwrap().accept().await }, if listener.is_some() => {
                let (stream, peer) = accepted.map_err(|_| ssh_error("IO_FAILED", "forwarding", "端口监听失败"))?;
                // 每条规则最多64条转发流，握手、建连也占额度；丢弃JoinSet会中止全部子任务。
                if tasks.len() >= 64 { continue; }
                let connection = connection.clone();
                let rule = rule.clone();
                tasks.spawn(async move { local(stream, peer, connection, rule).await; });
            },
            Some(request) = incoming.recv(), if rule.mode == ForwardMode::Remote => {
                if tasks.len() >= 64 { continue; }
                let host = rule.target_address.clone().unwrap();
                let port = rule.target_port.unwrap();
                tasks.spawn(async move {
                    let Ok(Ok(mut socket)) = tokio::time::timeout(Duration::from_secs(15), TcpStream::connect((host.as_str(), port))).await else { return; };
                    request.reply.accept().await;
                    let mut stream = request.channel.into_stream();
                    let _ = copy_bidirectional(&mut socket, &mut stream).await;
                });
            },
        }
    }
    // 退出或取消时连接的 TransportGuard 关闭 SSH 传输，同时释放远程监听。
}

async fn local(
    mut socket: TcpStream,
    peer: SocketAddr,
    connection: Arc<client::Handle<HostKeyHandler>>,
    rule: ForwardRule,
) {
    let destination = if rule.mode == ForwardMode::Dynamic {
        match tokio::time::timeout(Duration::from_secs(10), socks::request(&mut socket)).await {
            Ok(Ok(target)) => target,
            _ => return,
        }
    } else {
        (rule.target_address.unwrap(), rule.target_port.unwrap())
    };
    let channel = network(
        "forwarding",
        connection.channel_open_direct_tcpip(
            destination.0,
            u32::from(destination.1),
            peer.ip().to_string(),
            u32::from(peer.port()),
        ),
    )
    .await;
    let Ok(channel) = channel else {
        if rule.mode == ForwardMode::Dynamic {
            let _ = socks::reply(&mut socket, 5).await;
        }
        return;
    };
    if rule.mode == ForwardMode::Dynamic && socks::reply(&mut socket, 0).await.is_err() {
        return;
    }
    let mut stream = channel.into_stream();
    let _ = copy_bidirectional(&mut socket, &mut stream).await;
}

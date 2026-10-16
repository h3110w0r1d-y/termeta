use crate::ssh_error;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use terminal_core::*;

pub(crate) struct HostKeyHandler {
    pub remote_forward: Option<crate::forwarding::RemoteForward>,
    pub software: Arc<std::sync::Mutex<String>>,
    pub endpoint: HostEndpoint,
    pub trust: Arc<dyn HostTrustStore>,
    pub challenges: Arc<ChallengeBroker>,
    pub waiting: Arc<AtomicBool>,
    pub failure: Arc<std::sync::Mutex<Option<SessionError>>>,
}
impl russh::client::Handler for HostKeyHandler {
    type Error = russh::Error;
    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<russh::client::Msg>,
        connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: russh::client::ChannelOpenHandle,
        _session: &mut russh::client::Session,
    ) -> std::result::Result<(), Self::Error> {
        // 非远程转发连接、错误监听端点和满队列均通过丢弃reply拒绝，不阻塞SSH事件循环。
        if let Some(forward) = &self.remote_forward {
            if forward.matches(connected_address, connected_port) {
                let _ = forward
                    .incoming
                    .try_send(crate::forwarding::Incoming { channel, reply });
            }
        }
        Ok(())
    }
    async fn kex_done(
        &mut self,
        _shared_secret: Option<&[u8]>,
        _names: &russh::Names,
        session: &mut russh::client::Session,
    ) -> std::result::Result<(), Self::Error> {
        // 服务端标识只是识别线索，不作为信任、算法或权限依据。
        *self.software.lock().unwrap() = String::from_utf8_lossy(session.remote_sshid())
            .chars()
            .take(255)
            .collect();
        Ok(())
    }
    async fn check_server_key(
        &mut self,
        presented: &russh::keys::PublicKeyOrCertificate,
    ) -> std::result::Result<bool, Self::Error> {
        let result = self.check(presented).await;
        match result {
            Ok(()) => Ok(true),
            Err(error) => {
                *self.failure.lock().unwrap() = Some(error);
                Ok(false)
            }
        }
    }
}
impl HostKeyHandler {
    async fn check(&self, presented: &russh::keys::PublicKeyOrCertificate) -> Result<()> {
        if presented.certificate().is_some() {
            return Err(ssh_error(
                "UNSUPPORTED_OPERATION",
                "hostKey",
                "暂不支持主机证书；请使用普通主机公钥",
            ));
        }
        let public = presented.public_key();
        let key = HostKey {
            endpoint: self.endpoint.clone(),
            algorithm: public.algorithm().to_string(),
            public_key: public
                .to_bytes()
                .map_err(|_| ssh_error("HANDSHAKE_FAILED", "hostKey", "主机公钥编码失败"))?,
            fingerprint: public.fingerprint(russh::keys::HashAlg::Sha256).to_string(),
        };
        match self.trust.verify(&key).await? {
            TrustStatus::Trusted => Ok(()),
            TrustStatus::Changed => Err(ssh_error(
                "HOST_KEY_CHANGED",
                "hostKey",
                "主机公钥与已信任记录不符，已阻止连接；请先核验并管理信任记录",
            )),
            TrustStatus::Unknown => {
                self.waiting.store(true, Ordering::Release);
                let decision = self.challenges.host_key(&key).await;
                self.waiting.store(false, Ordering::Release);
                match decision? {
                    HostKeyDecision::Reject => {
                        Err(ssh_error("HOST_KEY_REJECTED", "hostKey", "已拒绝主机身份"))
                    }
                    HostKeyDecision::AllowOnce => {
                        if self.trust.verify(&key).await? == TrustStatus::Changed {
                            return Err(ssh_error(
                                "HOST_KEY_CHANGED",
                                "hostKey",
                                "等待确认期间主机信任记录已改变",
                            ));
                        }
                        Ok(())
                    }
                    HostKeyDecision::TrustAndSave => self.trust.trust(&key).await,
                }
            }
        }
    }
}

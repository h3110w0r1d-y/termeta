use crate::ssh_error;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use terminal_core::*;

pub(crate) struct HostKeyHandler {
    pub endpoint: HostEndpoint,
    pub trust: Arc<dyn HostTrustStore>,
    pub challenges: Arc<ChallengeBroker>,
    pub waiting: Arc<AtomicBool>,
    pub failure: Arc<std::sync::Mutex<Option<SessionError>>>,
}
impl russh::client::Handler for HostKeyHandler {
    type Error = russh::Error;
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

use crate::{host_key::HostKeyHandler, network};
use russh::{Channel, ChannelMsg, client};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use terminal_core::{Result, SessionError, SshConfig};
use tokio::sync::{mpsc, oneshot};

type Reply = oneshot::Sender<Result<Channel<client::Msg>>>;
type Registry = Arc<Mutex<HashMap<String, (String, EndpointIdentity, mpsc::Sender<Reply>)>>>;

/// 文件命名空间按连接地址、端口与用户识别；显示标题和共享主机密钥不代表同一端点。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndpointIdentity {
    host: String,
    port: u16,
    username: String,
}
impl From<&SshConfig> for EndpointIdentity {
    fn from(config: &SshConfig) -> Self {
        Self {
            host: config.endpoint().canonical_host,
            port: config.port,
            username: config.username.clone(),
        }
    }
}

/// 仅为已认证的会话签发子系统 channel；不向文件适配器暴露 SSH handle 或秘密。
#[derive(Clone, Default)]
pub struct SshChannels(Registry);
pub(crate) struct Registration {
    registry: Registry,
    id: String,
}
impl Drop for Registration {
    fn drop(&mut self) {
        self.registry.lock().unwrap().remove(&self.id);
    }
}
impl SshChannels {
    pub(crate) fn register(
        &self,
        id: &str,
        owner: &str,
        config: &SshConfig,
    ) -> (mpsc::Receiver<Reply>, Registration) {
        let (tx, rx) = mpsc::channel(2);
        self.0
            .lock()
            .unwrap()
            .insert(id.into(), (owner.into(), config.into(), tx));
        (
            rx,
            Registration {
                registry: self.0.clone(),
                id: id.into(),
            },
        )
    }
    pub fn endpoint_identity(&self, owner: &str, id: &str) -> Result<EndpointIdentity> {
        self.0
            .lock()
            .unwrap()
            .get(id)
            .filter(|(stored, _, _)| stored == owner)
            .map(|(_, identity, _)| identity.clone())
            .ok_or_else(unavailable)
    }
    pub async fn open_sftp(
        &self,
        owner: &str,
        id: &str,
    ) -> Result<russh::ChannelStream<client::Msg>> {
        let sender = self
            .0
            .lock()
            .unwrap()
            .get(id)
            .filter(|(stored, _, _)| stored == owner)
            .map(|(_, _, tx)| tx.clone())
            .ok_or_else(unavailable)?;
        let (tx, rx) = oneshot::channel();
        sender.try_send(tx).map_err(|_| unavailable())?;
        let mut channel = tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| unavailable())?
            .map_err(|_| unavailable())??;
        network("sftp", channel.request_subsystem(true, "sftp")).await?;
        tokio::time::timeout(std::time::Duration::from_secs(15), async {
            loop {
                match channel.wait().await {
                    Some(ChannelMsg::Success) => return Ok(()),
                    Some(ChannelMsg::Failure | ChannelMsg::Close | ChannelMsg::Eof) | None => {
                        return Err(unavailable());
                    }
                    // 子系统接受前不应有协议数据，避免丢弃后继续使用损坏的流。
                    Some(ChannelMsg::Data { .. } | ChannelMsg::ExtendedData { .. }) => {
                        return Err(unavailable());
                    }
                    _ => {}
                }
            }
        })
        .await
        .map_err(|_| unavailable())??;
        Ok(channel.into_stream())
    }
}
fn unavailable() -> SessionError {
    SessionError::new(
        "SFTP_UNAVAILABLE",
        "sftp",
        "SFTP 不可用：连接已结束、请求繁忙或服务器拒绝子系统",
    )
}

pub(crate) async fn serve(
    connection: &client::Handle<HostKeyHandler>,
    mut requests: mpsc::Receiver<Reply>,
) {
    while let Some(reply) = requests.recv().await {
        if reply.is_closed() {
            continue;
        }
        let result = network("sftp", connection.channel_open_session()).await;
        let _ = reply.send(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(address: &str, port: u16, username: &str) -> SshConfig {
        SshConfig {
            system_profile_id: None,
            config_version: 1,
            address: address.into(),
            port,
            username: username.into(),
            auth_plan: vec![],
            startup: Default::default(),
        }
    }

    #[test]
    fn endpoint_identity_keeps_port_and_user_but_normalizes_host() {
        let original = config("Example.Invalid.", 22, "user");
        let same = config("example.invalid", 22, "user");
        assert_eq!(
            EndpointIdentity::from(&original),
            EndpointIdentity::from(&same)
        );
        for distinct in [
            config("example.invalid", 2222, "user"),
            config("example.invalid", 22, "other"),
            config("other.invalid", 22, "user"),
        ] {
            assert_ne!(
                EndpointIdentity::from(&original),
                EndpointIdentity::from(&distinct)
            );
        }
        assert_eq!(
            EndpointIdentity::from(&config("2001:0db8::1", 22, "user")),
            EndpointIdentity::from(&config("2001:db8::1", 22, "user")),
        );
    }

    #[test]
    fn endpoint_identity_is_owner_bound_and_removed_with_registration() {
        let channels = SshChannels::default();
        let (_, registration) = channels.register("session", "main", &config("host", 22, "user"));
        assert!(channels.endpoint_identity("settings", "session").is_err());
        assert!(channels.endpoint_identity("main", "session").is_ok());
        drop(registration);
        assert!(channels.endpoint_identity("main", "session").is_err());
    }
}

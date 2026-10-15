use crate::{host_key::HostKeyHandler, network};
use russh::{Channel, ChannelMsg, client};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use terminal_core::{Result, SessionError};
use tokio::sync::{mpsc, oneshot};

type Reply = oneshot::Sender<Result<Channel<client::Msg>>>;
type Registry = Arc<Mutex<HashMap<String, (String, mpsc::Sender<Reply>)>>>;

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
    pub(crate) fn register(&self, id: &str, owner: &str) -> (mpsc::Receiver<Reply>, Registration) {
        let (tx, rx) = mpsc::channel(2);
        self.0.lock().unwrap().insert(id.into(), (owner.into(), tx));
        (
            rx,
            Registration {
                registry: self.0.clone(),
                id: id.into(),
            },
        )
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
            .filter(|(stored, _)| stored == owner)
            .map(|(_, tx)| tx.clone())
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

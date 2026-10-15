use crate::*;
use async_trait::async_trait;

#[derive(Clone)]
pub struct HostKey {
    pub endpoint: HostEndpoint,
    pub algorithm: String,
    pub public_key: Vec<u8>,
    pub fingerprint: String,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrustStatus {
    Trusted,
    Unknown,
    Changed,
}
#[async_trait]
pub trait HostTrustStore: Send + Sync {
    async fn verify(&self, key: &HostKey) -> Result<TrustStatus>;
    /// 实现必须在事务内重查 endpoint，禁止并发确认覆盖已有的不同密钥。
    async fn trust(&self, key: &HostKey) -> Result<()>;
}
#[async_trait]
pub trait CredentialStore: Send + Sync {
    async fn secret(&self, owner: &str, credential_id: &str) -> Result<Zeroizing<String>>;
    /// 返回已授权且限大小读取的私钥文本，不能把前端任意路径作为引用。
    async fn key_file(&self, owner: &str, key_file_ref: &str) -> Result<Zeroizing<String>>;
}
pub trait SessionSink: Send + Sync {
    fn output(&self, frame: Vec<u8>) -> Result<()>;
    fn event(&self, event: SessionEvent) -> Result<()>;
}
#[async_trait]
pub trait SessionFactory: Send + Sync {
    fn kind(&self) -> &'static str;
    fn capabilities(&self) -> SessionCapabilities;
    fn validate(&self, config: &SshConfig) -> Result<()>;
    async fn run(&self, config: SshConfig, context: DriverContext) -> Result<SessionEnd>;
}

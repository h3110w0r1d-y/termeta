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
pub struct PrivateKeyMaterial {
    pub private_key: Zeroizing<String>,
    pub passphrase: Option<Zeroizing<String>>,
}
#[async_trait]
pub trait CredentialStore: Send + Sync {
    async fn secret(&self, owner: &str, credential_id: &str) -> Result<Zeroizing<String>>;
    /// 私钥与可选解密口令归属于同一个凭据，不向界面返回秘密。
    async fn private_key(&self, owner: &str, credential_id: &str) -> Result<PrivateKeyMaterial>;
}
pub trait SessionSink: Send + Sync {
    fn output(&self, frame: Vec<u8>) -> Result<()>;
    fn event(&self, event: SessionEvent) -> Result<()>;
}
#[async_trait]
pub trait SessionFactory: Send + Sync {
    fn kind(&self) -> &'static str;
    fn capabilities(&self) -> SessionCapabilities;
    fn validate(&self, config: &SessionConfig) -> Result<()>;
    async fn run(&self, config: SessionConfig, context: DriverContext) -> Result<SessionEnd>;
}

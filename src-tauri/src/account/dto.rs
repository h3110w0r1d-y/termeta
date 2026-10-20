//! 账号边界显式区分服务端命名、持久化秘密与 WebView 摘要。
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUser {
    pub id: String,
    pub email: String,
    #[serde(alias = "is_admin")]
    pub is_admin: bool,
    #[serde(alias = "has_password")]
    pub has_password: bool,
    #[serde(alias = "created_at")]
    pub created_at: String,
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    pub user: Option<AccountUser>,
    pub expires_at: Option<String>,
}
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Termeta,
    Github,
    Google,
}
impl Provider {
    pub fn id(self) -> &'static str {
        match self {
            Self::Termeta => "termeta",
            Self::Github => "github",
            Self::Google => "google",
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct AccountProvider {
    pub id: Provider,
    pub name: String,
}
#[derive(Deserialize)]
pub struct Providers {
    pub providers: Vec<AccountProvider>,
}
#[derive(Deserialize)]
pub struct TokenView {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: Option<String>,
    pub user: AccountUser,
}
impl Drop for TokenView {
    fn drop(&mut self) {
        self.access_token.zeroize();
    }
}
#[derive(Deserialize)]
pub struct StartView {
    pub flow_id: String,
    pub authorize_url: String,
    pub expires_at: String,
    pub poll_interval: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStarted {
    pub flow_id: String,
    pub provider: Provider,
    pub expires_at: String,
    pub poll_interval: u64,
}
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FlowStatus {
    Pending,
    Completed,
    Denied,
    Expired,
}
#[derive(Deserialize)]
pub struct PollView {
    pub status: FlowStatus,
    pub token: Option<TokenView>,
    pub error: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthPolled {
    pub status: FlowStatus,
    pub account: Option<AccountStatus>,
    pub error: Option<String>,
}

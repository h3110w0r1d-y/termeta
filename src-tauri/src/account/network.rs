//! 所有账号请求绑定编译期官方来源，不接受 WebView 提供的 URL。
use super::{dto::Provider, failure};
use reqwest::{Client, Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;
use terminal_core::Result;
use url::Url;
use zeroize::Zeroizing;

pub struct Api {
    client: Client,
    pub origin: String,
    base: Url,
}
impl Api {
    pub fn new() -> Result<Self> {
        let mut base =
            Url::parse(option_env!("TERMETA_API_BASE_URL").unwrap_or("https://termeta.io"))
                .map_err(|_| failure("ACCOUNT_CONFIG", "官方服务地址配置无效"))?;
        if base.scheme() != "https"
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || !matches!(base.path(), "" | "/")
        {
            return Err(failure("ACCOUNT_CONFIG", "官方服务必须使用固定 HTTPS 来源"));
        }
        base.set_path("/");
        let origin = base.as_str().to_owned();
        let client = Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(25))
            .user_agent(concat!("Termeta/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| failure("ACCOUNT_NETWORK", "无法初始化官方服务连接"))?;
        Ok(Self {
            client,
            origin,
            base,
        })
    }
    pub async fn request<T: DeserializeOwned, B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
        body: Option<&B>,
    ) -> Result<T> {
        let bytes = self.send(method, path, token, body).await?;
        serde_json::from_slice(&bytes)
            .map_err(|_| failure("ACCOUNT_RESPONSE", "官方服务响应格式无效"))
    }
    pub async fn send<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
        body: Option<&B>,
    ) -> Result<Zeroizing<Vec<u8>>> {
        let url = self
            .base
            .join(path)
            .map_err(|_| failure("ACCOUNT_CONFIG", "官方服务路径无效"))?;
        let mut request = self
            .client
            .request(method, url)
            .header("Accept", "application/json");
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| failure("ACCOUNT_NETWORK", "无法连接官方服务，请检查网络后重试"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(match status {
                StatusCode::UNAUTHORIZED => {
                    failure("ACCOUNT_UNAUTHORIZED", "登录已失效，或邮箱密码不正确")
                }
                StatusCode::FORBIDDEN => failure("ACCOUNT_FORBIDDEN", "此账号不能执行该操作"),
                StatusCode::CONFLICT => failure(
                    "ACCOUNT_CONFLICT",
                    "账号或登录方式存在冲突，请使用已有账号登录后绑定",
                ),
                StatusCode::TOO_MANY_REQUESTS => {
                    failure("ACCOUNT_RATE_LIMITED", "操作过于频繁，请稍后重试")
                }
                StatusCode::NOT_FOUND => failure("ACCOUNT_NOT_FOUND", "登录方式未启用或授权已过期"),
                StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
                    failure("ACCOUNT_INVALID", "账号信息或登录方式无效，请检查后重试")
                }
                _ => failure("ACCOUNT_UNAVAILABLE", "官方服务暂时不可用，请稍后重试"),
            });
        }
        let limit = if path == "api/sync/vault" {
            48 * 1024 * 1024
        } else {
            64 * 1024
        };
        if response
            .content_length()
            .is_some_and(|size| size > limit as u64)
        {
            return Err(failure("ACCOUNT_RESPONSE", "官方服务响应超出限制"));
        }
        let mut bytes = Zeroizing::new(Vec::new());
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| failure("ACCOUNT_NETWORK", "官方服务响应中断"))?
        {
            if bytes.len().saturating_add(chunk.len()) > limit {
                return Err(failure("ACCOUNT_RESPONSE", "官方服务响应超出限制"));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}
pub fn authorization_url(value: &str, provider: Provider) -> Result<Url> {
    if value.len() > 8192 || value.chars().any(char::is_control) {
        return Err(failure("ACCOUNT_RESPONSE", "提供方授权地址无效"));
    }
    let url = Url::parse(value).map_err(|_| failure("ACCOUNT_RESPONSE", "提供方授权地址无效"))?;
    let valid_endpoint = match provider {
        Provider::Termeta => {
            let api = Api::new()?;
            url.origin() == api.base.origin() && url.path() == "/desktop/authorize"
        }
        Provider::Github => {
            url.host_str() == Some("github.com") && url.path() == "/login/oauth/authorize"
        }
        Provider::Google => {
            url.host_str() == Some("accounts.google.com") && url.path() == "/o/oauth2/v2/auth"
        }
    };
    if !valid_endpoint
        || url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(failure("ACCOUNT_RESPONSE", "提供方授权地址无效"));
    }
    Ok(url)
}

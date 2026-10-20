//! 账号用例不依赖窗口句柄；调用方负责本地资源权限与失效通知。
use super::{
    dto::*,
    failure,
    network::{Api, authorization_url},
    runtime::{Flow, Stamp, authorization_deadline},
    storage::{self, SavedAccount},
};
use crate::state::AppState;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand_core::{OsRng, RngCore};
use reqwest::Method;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Instant;
use terminal_core::Result;
use zeroize::Zeroizing;

fn check(state: &AppState, stamp: &Stamp) -> Result<()> {
    state.account.check(stamp)?;
    Ok(())
}
async fn revoke(api: &Api, token: &str) -> Result<()> {
    api.send::<()>(Method::POST, "api/auth/logout", Some(token), None)
        .await
        .map(|_| ())
}
async fn install(
    state: &AppState,
    api: &Api,
    stamp: &Stamp,
    token: &TokenView,
    flow_id: Option<(&str, &str)>,
) -> Result<AccountStatus> {
    let account = match SavedAccount::from_token(&api.origin, token) {
        Ok(account) => account,
        Err(error) => {
            let _ = revoke(api, &token.access_token).await;
            return Err(error);
        }
    };
    let result = async {
        let _commit = state.account.commit.lock().await;
        check(state, stamp)?;
        if flow_id.is_some_and(|(owner, id)| !state.account.contains(owner, id)) {
            return Err(failure("ACCOUNT_CANCELLED", "浏览器登录已取消"));
        }
        storage::save(state, Some(&account)).await?;
        // 数据库等待期间也可能发生退出、销毁或取消，提交后再次校验并清除迟到会话。
        if check(state, stamp).is_err()
            || flow_id.is_some_and(|(owner, id)| !state.account.contains(owner, id))
        {
            storage::save(state, None).await?;
            return Err(failure("ACCOUNT_CANCELLED", "账号操作已取消"));
        }
        // 会话成功切换后，其他窗口尚未完成的登录全部失效。
        state.account.invalidate();
        state.official.reset();
        Ok(account.status())
    }
    .await;
    if result.is_err() {
        let _ = revoke(api, &account.access_token).await;
    }
    result
}
pub async fn status(state: &AppState) -> Result<AccountStatus> {
    let api = Api::new()?;
    let _commit = state.account.commit.lock().await;
    Ok(storage::load(state, &api.origin)
        .await?
        .as_ref()
        .map(SavedAccount::status)
        .unwrap_or_default())
}
pub async fn refresh(state: &AppState, owner: &str) -> Result<AccountStatus> {
    let _sync = state
        .official
        .operation
        .try_lock()
        .map_err(|_| failure("ACCOUNT_BUSY", "账号或同步操作正在进行"))?;
    let stamp = state.account.stamp(owner);
    let _operation = state
        .account
        .operation
        .try_lock()
        .map_err(|_| failure("ACCOUNT_BUSY", "账号操作正在进行"))?;
    let api = Api::new()?;
    let Some(mut account) = storage::load(state, &api.origin).await? else {
        return Ok(AccountStatus::default());
    };
    let user = api
        .request::<AccountUser, ()>(
            Method::GET,
            "api/auth/me",
            Some(&account.access_token),
            None,
        )
        .await;
    let _commit = state.account.commit.lock().await;
    check(state, &stamp)?;
    match user {
        Ok(user) => {
            account.user = user;
            storage::save(state, Some(&account)).await?;
            if check(state, &stamp).is_err() {
                storage::save(state, None).await?;
                check(state, &stamp)?;
            }
            Ok(account.status())
        }
        Err(error) => {
            if error.code == "ACCOUNT_UNAUTHORIZED" {
                storage::save(state, None).await?;
            }
            Err(error)
        }
    }
}
pub async fn logout_local(state: &AppState) -> Result<Option<SavedAccount>> {
    // 不等待正在联网的账号操作；代次先失效，持久化清理再短暂串行。
    state.account.invalidate();
    let _commit = state.account.commit.lock().await;
    storage::take_for_logout(state).await
}
pub async fn logout_remote(account: Option<SavedAccount>) -> Result<()> {
    if let Some(account) = account {
        let api = Api::new()?;
        // 编译期来源发生变化时绝不把旧凭据发给新地址。
        if account.origin != api.origin {
            return Ok(());
        }
        revoke(&api, &account.access_token).await.map_err(|_| {
            failure(
                "ACCOUNT_LOGOUT_REMOTE",
                "已退出本机账号，但远端会话未能撤销，请稍后在服务端检查会话",
            )
        })?;
    }
    Ok(())
}
pub async fn providers() -> Result<Vec<AccountProvider>> {
    let result: Providers = Api::new()?
        .request::<_, ()>(Method::GET, "api/auth/providers", None, None)
        .await?;
    if result.providers.len() > 3 {
        return Err(failure("ACCOUNT_RESPONSE", "登录方式响应无效"));
    }
    Ok(result.providers)
}
pub async fn start(
    state: &AppState,
    owner: &str,
    provider: Provider,
) -> Result<(OAuthStarted, url::Url)> {
    let stamp = state.account.stamp(owner);
    let _operation = state
        .account
        .operation
        .try_lock()
        .map_err(|_| failure("ACCOUNT_BUSY", "账号操作正在进行"))?;
    let api = Api::new()?;
    if storage::load(state, &api.origin).await?.is_some() {
        return Err(failure(
            "ACCOUNT_ALREADY_SIGNED_IN",
            "请先退出当前账号再登录其他账号",
        ));
    }
    let mut random = Zeroizing::new([0_u8; 32]);
    OsRng
        .try_fill_bytes(&mut *random)
        .map_err(|_| failure("ACCOUNT_RANDOM", "无法生成安全登录凭证"))?;
    let verifier = Zeroizing::new(URL_SAFE_NO_PAD.encode(&*random));
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let path = format!("api/auth/oauth/{}/start", provider.id());
    let response: StartView = api
        .request(
            Method::POST,
            &path,
            None,
            Some(&json!({"code_challenge": challenge, "code_challenge_method": "S256"})),
        )
        .await?;
    let deadline = authorization_deadline(&response.expires_at);
    let flow = Flow {
        provider,
        verifier,
        confirmation_code: None,
        stamp: stamp.clone(),
        deadline: deadline
            .as_ref()
            .copied()
            .unwrap_or_else(|_| Instant::now()),
        last_poll: None,
    };
    let valid_response = uuid::Uuid::parse_str(&response.flow_id)
        .is_ok_and(|id| id.to_string() == response.flow_id)
        && response.expires_at.len() <= 64
        && deadline.is_ok()
        && (3..=30).contains(&response.poll_interval);
    let url = authorization_url(&response.authorize_url, provider);
    if !valid_response || url.is_err() || check(state, &stamp).is_err() {
        let _ = cancel_remote(&api, &response.flow_id, &flow).await;
        return Err(failure(
            "ACCOUNT_CANCELLED",
            "登录已取消或官方服务授权响应无效",
        ));
    }
    if let Err(error) = state.account.insert(response.flow_id.clone(), flow.clone()) {
        let _ = cancel_remote(&api, &response.flow_id, &flow).await;
        return Err(error);
    }
    Ok((
        OAuthStarted {
            flow_id: response.flow_id,
            provider,
            expires_at: response.expires_at,
            poll_interval: response.poll_interval,
        },
        url?,
    ))
}
async fn cancel_remote(api: &Api, id: &str, flow: &Flow) -> Result<()> {
    #[derive(serde::Serialize)]
    struct Proof<'a> {
        flow_id: &'a str,
        code_verifier: &'a str,
    }
    api.send(
        Method::POST,
        &format!("api/auth/oauth/{}/cancel", flow.provider.id()),
        None,
        Some(&Proof {
            flow_id: id,
            code_verifier: &flow.verifier,
        }),
    )
    .await
    .map(|_| ())
}
pub async fn cancel(state: &AppState, owner: &str, id: &str) -> Result<()> {
    let Some(flow) = state.account.remove(owner, id) else {
        return Ok(());
    };
    cancel_remote(&Api::new()?, id, &flow).await
}
pub async fn poll(state: &AppState, owner: &str, id: &str) -> Result<OAuthPolled> {
    let _operation = state
        .account
        .operation
        .try_lock()
        .map_err(|_| failure("ACCOUNT_BUSY", "账号操作正在进行"))?;
    let flow = state.account.flow(owner, id, true)?;
    let api = Api::new()?;
    #[derive(serde::Serialize)]
    struct Proof<'a> {
        flow_id: &'a str,
        code_verifier: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        confirmation_code: Option<&'a str>,
    }
    let result: PollView = api
        .request(
            Method::POST,
            &format!("api/auth/oauth/{}/poll", flow.provider.id()),
            None,
            Some(&Proof {
                flow_id: id,
                code_verifier: &flow.verifier,
                confirmation_code: flow.confirmation_code.as_ref().map(|code| code.as_str()),
            }),
        )
        .await?;
    if check(state, &flow.stamp).is_err() || !state.account.contains(owner, id) {
        if let Some(token) = &result.token {
            let _ = revoke(&api, &token.access_token).await;
        }
        return Err(failure("ACCOUNT_CANCELLED", "浏览器登录已取消"));
    }
    if result.status == FlowStatus::Completed && flow.confirmation_code.is_none() {
        state.account.remove(owner, id);
        if let Some(token) = &result.token {
            let _ = revoke(&api, &token.access_token).await;
        }
        return Err(failure(
            "ACCOUNT_RESPONSE",
            "官方服务未按确认码协议完成授权，请重新发起",
        ));
    }
    let account = if result.status == FlowStatus::Completed {
        let token = result
            .token
            .as_ref()
            .ok_or_else(|| failure("ACCOUNT_RESPONSE", "登录响应缺少账号凭证"))?;
        Some(install(state, &api, &flow.stamp, token, Some((owner, id))).await?)
    } else {
        None
    };
    if result.status != FlowStatus::Pending {
        state.account.remove(owner, id);
    }
    Ok(OAuthPolled {
        status: result.status,
        account,
        // 服务端公开诊断保留可操作信息；只接收有界可见文本，拒绝异常载荷。
        error: result.error.filter(|message| {
            !message.is_empty()
                && message.chars().count() <= 512
                && !message.chars().any(char::is_control)
        }),
    })
}

//! 独立 settings.account 仅存于 SQLCipher，业务备份白名单不会导出此设置。
use super::{
    dto::{AccountStatus, AccountUser, TokenView},
    failure,
};
use crate::state::{AppState, storage_error};
use serde::{Deserialize, Serialize};
use terminal_core::Result;
use zeroize::Zeroize;

const KEY: &str = "account";
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedAccount {
    pub origin: String,
    pub access_token: String,
    pub expires_at: Option<String>,
    pub user: AccountUser,
    #[serde(default)]
    pub sync_confirmed: bool,
}
impl Drop for SavedAccount {
    fn drop(&mut self) {
        self.access_token.zeroize();
    }
}
impl SavedAccount {
    pub fn status(&self) -> AccountStatus {
        AccountStatus {
            user: Some(self.user.clone()),
            expires_at: self.expires_at.clone(),
        }
    }
    pub fn from_token(origin: &str, token: &TokenView) -> Result<Self> {
        if !token.token_type.eq_ignore_ascii_case("bearer")
            || token.access_token.is_empty()
            || token.access_token.len() > 8192
            || token.access_token.chars().any(char::is_control)
            || token.user.id.len() > 128
            || token.user.email.len() > 254
            || token
                .expires_at
                .as_ref()
                .is_some_and(|value| value.len() > 64)
        {
            return Err(failure("ACCOUNT_RESPONSE", "官方服务账号响应无效"));
        }
        Ok(Self {
            origin: origin.into(),
            access_token: token.access_token.clone(),
            expires_at: token.expires_at.clone(),
            user: token.user.clone(),
            sync_confirmed: false,
        })
    }
}
pub async fn load(state: &AppState, origin: &str) -> Result<Option<SavedAccount>> {
    if let Some(account) = state.account.pending.lock().unwrap().as_ref() {
        if account.origin != origin {
            return Err(failure("ACCOUNT_ORIGIN_CHANGED", "官方服务来源已变化"));
        }
        return Ok(Some(account.clone()));
    }
    if !state.vault.is_unlocked() {
        return Ok(None);
    }
    let Some(mut setting) = state.store.get_setting(KEY).await.map_err(storage_error)? else {
        return Ok(None);
    };
    if setting.value_version != 1 {
        return Err(failure(
            "ACCOUNT_STORAGE_VERSION",
            "账号设置版本不受支持，已停止读取与覆盖",
        ));
    }
    if setting.value.is_null() {
        return Ok(None);
    }
    let result = serde_json::from_value::<SavedAccount>(setting.value.take())
        .map_err(|_| failure("ACCOUNT_STORAGE", "本机账号信息无效"))?;
    if result.origin != origin {
        return Err(failure(
            "ACCOUNT_ORIGIN_CHANGED",
            "本机账号属于其他官方服务来源，请先退出账号",
        ));
    }
    Ok(Some(result))
}
pub async fn save(state: &AppState, account: Option<&SavedAccount>) -> Result<()> {
    if !state.vault.is_unlocked() {
        *state.account.pending.lock().unwrap() = account.cloned();
        return Ok(());
    }
    let mut value = serde_json::to_value(account)
        .map_err(|_| failure("ACCOUNT_STORAGE", "无法保存本机账号"))?;
    let result = state
        .store
        .set_setting(KEY, 1, &value)
        .await
        .map_err(storage_error);
    if let Some(secret) = value.get_mut("accessToken") {
        if let serde_json::Value::String(secret) = secret {
            secret.zeroize();
        }
    }
    if result.is_ok() {
        *state.account.pending.lock().unwrap() = None;
    }
    result
}

/// 显式退出可丢弃损坏的当前版本记录，但不能覆盖未来版本格式。
pub async fn take_for_logout(state: &AppState) -> Result<Option<SavedAccount>> {
    if !state.vault.is_unlocked() {
        return Ok(state.account.pending.lock().unwrap().take());
    }
    let setting = state.store.get_setting(KEY).await.map_err(storage_error)?;
    if setting
        .as_ref()
        .is_some_and(|setting| setting.value_version != 1)
    {
        return Err(failure(
            "ACCOUNT_STORAGE_VERSION",
            "账号设置版本不受支持，已停止覆盖",
        ));
    }
    let stored = setting
        .and_then(|mut setting| serde_json::from_value::<SavedAccount>(setting.value.take()).ok());
    let pending = state.account.pending.lock().unwrap().clone();
    // setup/locked 登录后可能先解锁本地库再退出，必须清理仍在内存的当前会话。
    save(state, None).await?;
    Ok(pending.or(stored))
}

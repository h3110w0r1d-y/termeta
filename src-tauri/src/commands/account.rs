//! 账号命令只负责受信任窗口边界、系统浏览器与失效通知。
use crate::{
    account::{dto::*, failure, service},
    state::{AppState, settings_owner},
};
use tauri::{Emitter, Manager, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;
use terminal_core::Result;

fn changed(window: &WebviewWindow) {
    let _ = window.app_handle().emit("account-changed", ());
}
#[tauri::command]
pub async fn get_account_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<AccountStatus> {
    settings_owner(&window)?;
    service::status(&state).await
}
#[tauri::command]
pub async fn refresh_account(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<AccountStatus> {
    settings_owner(&window)?;
    let result = service::refresh(&state, window.label()).await;
    changed(&window);
    result
}
#[tauri::command]
pub async fn list_account_providers(window: WebviewWindow) -> Result<Vec<AccountProvider>> {
    settings_owner(&window)?;
    service::providers().await
}
#[tauri::command]
pub async fn register_account(
    window: WebviewWindow,
    state: State<'_, AppState>,
    email: String,
    password: String,
) -> Result<AccountStatus> {
    let password = zeroize::Zeroizing::new(password);
    settings_owner(&window)?;
    let result = service::authenticate(&state, window.label(), email, password, true).await;
    changed(&window);
    result
}
#[tauri::command]
pub async fn login_account(
    window: WebviewWindow,
    state: State<'_, AppState>,
    email: String,
    password: String,
) -> Result<AccountStatus> {
    let password = zeroize::Zeroizing::new(password);
    settings_owner(&window)?;
    let result = service::authenticate(&state, window.label(), email, password, false).await;
    changed(&window);
    result
}
#[tauri::command]
pub async fn logout_account(window: WebviewWindow, state: State<'_, AppState>) -> Result<()> {
    settings_owner(&window)?;
    let account = service::logout_local(&state).await?;
    // 本机秘密已清除，先通知其他窗口，再等待可能失败的远端撤销。
    changed(&window);
    service::logout_remote(account).await
}
#[tauri::command]
pub async fn start_account_oauth(
    window: WebviewWindow,
    state: State<'_, AppState>,
    provider: Provider,
    link: Option<bool>,
) -> Result<OAuthStarted> {
    settings_owner(&window)?;
    let (started, url) =
        service::start(&state, window.label(), provider, link.unwrap_or(false)).await?;
    // 开浏览器前再次校验窗口仍存在；URL 已在域层限制为两个固定提供方端点。
    let live = window
        .app_handle()
        .get_webview_window(window.label())
        .is_some()
        && state.account.contains(window.label(), &started.flow_id);
    if !live
        || window
            .opener()
            .open_url(url.as_str(), None::<&str>)
            .is_err()
    {
        let _ = service::cancel(&state, window.label(), &started.flow_id).await;
        return Err(failure(
            "ACCOUNT_BROWSER",
            "无法打开系统浏览器，请重新发起登录",
        ));
    }
    Ok(started)
}
#[tauri::command]
pub fn confirm_account_oauth(
    window: WebviewWindow,
    state: State<'_, AppState>,
    flow_id: String,
    confirmation_code: String,
) -> Result<()> {
    let confirmation_code = zeroize::Zeroizing::new(confirmation_code);
    settings_owner(&window)?;
    state
        .account
        .confirm(window.label(), &flow_id, confirmation_code)
}
#[tauri::command]
pub async fn poll_account_oauth(
    window: WebviewWindow,
    state: State<'_, AppState>,
    flow_id: String,
) -> Result<OAuthPolled> {
    settings_owner(&window)?;
    let result = service::poll(&state, window.label(), &flow_id).await;
    if result
        .as_ref()
        .is_ok_and(|value| value.status == FlowStatus::Completed)
    {
        changed(&window);
    }
    result
}
#[tauri::command]
pub async fn cancel_account_oauth(
    window: WebviewWindow,
    state: State<'_, AppState>,
    flow_id: String,
) -> Result<()> {
    settings_owner(&window)?;
    service::cancel(&state, window.label(), &flow_id).await
}
#[tauri::command]
pub async fn list_account_identities(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<AccountIdentity>> {
    settings_owner(&window)?;
    service::identities(&state, window.label()).await
}
#[tauri::command]
pub async fn unlink_account_identity(
    window: WebviewWindow,
    state: State<'_, AppState>,
    provider: Provider,
) -> Result<()> {
    settings_owner(&window)?;
    let result = service::unlink(&state, window.label(), provider).await;
    if result.is_ok() {
        changed(&window);
    }
    result
}

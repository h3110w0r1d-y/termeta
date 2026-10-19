//! 系统协议只把一次性确认码交给本机在途授权，不承接通用导航或命令。

use super::dto::Provider;
use crate::state::{AppState, settings_owner};
use tauri::{AppHandle, Manager, Runtime, plugin::Plugin};
#[cfg(any(target_os = "linux", all(debug_assertions, target_os = "windows")))]
use tauri_plugin_deep_link::DeepLinkExt;
use zeroize::Zeroizing;

/// 仅复用官方注册初始化；系统回跳通过原生入口直接进入 Rust。
/// 官方默认事件会向 WebView 广播原始 URI，因此不转发其事件和 JS API 钩子。
struct RegistrationOnly<R: Runtime> {
    inner: Box<dyn Plugin<R>>,
}

impl<R: Runtime> Plugin<R> for RegistrationOnly<R> {
    fn name(&self) -> &'static str {
        "deep-link"
    }

    fn initialize(
        &mut self,
        app: &AppHandle<R>,
        _config: serde_json::Value,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // 安装包仍读取 tauri.conf 的 scheme；运行期传 None，禁止初始化扫描 argv 并广播 URI。
        self.inner.initialize(app, serde_json::Value::Null)
    }
}

pub fn registration_plugin<R: Runtime>() -> impl Plugin<R> {
    RegistrationOnly {
        inner: Box::new(tauri_plugin_deep_link::init()),
    }
}

pub fn register(app: &AppHandle) {
    // 安装包声明处理器；Linux 另覆盖 AppImage，Windows 仅在开发时注册当前程序。
    #[cfg(any(target_os = "linux", all(debug_assertions, target_os = "windows")))]
    if app.deep_link().register("termeta").is_err() {
        eprintln!("Termeta 协议注册未完成，可使用浏览器一次性确认码返回登录");
    }
    #[cfg(not(any(target_os = "linux", all(debug_assertions, target_os = "windows"))))]
    let _ = app;
}

struct AuthorizationReturn {
    flow_id: String,
    provider: Provider,
    confirmation_code: Zeroizing<String>,
}

impl AuthorizationReturn {
    fn parse(raw: &str) -> Option<Self> {
        if raw.len() > 512 || !raw.is_ascii() || raw.bytes().any(|byte| byte.is_ascii_control()) {
            return None;
        }
        let url = url::Url::parse(raw).ok()?;
        if url.as_str() != raw
            || url.scheme() != "termeta"
            || url.host_str() != Some("auth")
            || url.path() != "/callback"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
        {
            return None;
        }
        let mut flow_id = None;
        let mut provider = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "flow_id" if flow_id.is_none() => {
                    let id = uuid::Uuid::parse_str(&value).ok()?;
                    if id.to_string() != value.as_ref() {
                        return None;
                    }
                    flow_id = Some(id.to_string());
                }
                "provider" if provider.is_none() => {
                    provider = Some(match value.as_ref() {
                        "github" => Provider::Github,
                        "google" => Provider::Google,
                        _ => return None,
                    });
                }
                _ => return None,
            }
        }
        let code = url.fragment()?.strip_prefix("confirmation_code=")?;
        if code.len() != 32
            || !code
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return None;
        }
        Some(Self {
            flow_id: flow_id?,
            provider: provider?,
            confirmation_code: Zeroizing::new(code.to_owned()),
        })
    }
}

/// 无匹配 flow 时直接拒绝，不建立新授权、不联网、不导航 WebView。
pub fn receive(app: &AppHandle, raw: &str) -> bool {
    let Some(callback) = AuthorizationReturn::parse(raw) else {
        return false;
    };
    let Some(state) = app.try_state::<AppState>() else {
        return false;
    };
    if !state.vault.is_unlocked() {
        state.account.invalidate();
        return false;
    }
    let Ok(owner) = state
        .account
        .callback_owner(&callback.flow_id, callback.provider)
    else {
        return false;
    };
    let Some(window) = app.get_webview_window(&owner) else {
        state.account.clear_owner(&owner);
        return false;
    };
    if settings_owner(&window).is_err()
        || state
            .account
            .confirm_callback(
                &owner,
                &callback.flow_id,
                callback.provider,
                callback.confirmation_code,
            )
            .is_err()
    {
        return false;
    }
    // 正常轮询已由发起窗口持有，回跳只交接确认码，不启动第二个兑换请求。
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    true
}

pub fn second_instance(app: &AppHandle, args: &[String]) -> bool {
    if args.len() != 2 {
        return false;
    }
    receive(app, &args[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn callback() -> String {
        format!(
            "termeta://auth/callback?flow_id=11111111-1111-4111-8111-111111111111&provider=github#confirmation_code={}",
            "C".repeat(32)
        )
    }

    #[test]
    fn fixed_callback_accepts_only_a_confirmation_not_an_application_session() {
        let parsed = AuthorizationReturn::parse(&callback()).unwrap();
        assert!(parsed.provider == Provider::Github);
        assert_eq!(parsed.confirmation_code.as_str(), "C".repeat(32));
        assert_eq!(parsed.flow_id, "11111111-1111-4111-8111-111111111111");
    }

    #[test]
    fn rejects_ambiguous_or_unrelated_external_input() {
        let valid = callback();
        for invalid in [
            valid.replace("termeta:", "https:"),
            valid.replace("//auth/", "//attacker/"),
            valid.replace("//auth/", "//user@auth/"),
            valid.replace("//auth/", "//auth:123/"),
            valid.replace("/callback?", "/other?"),
            valid.replace("/callback?", "/other/../callback?"),
            valid.replace("provider=github", "provider=github&provider=github"),
            valid.replace("provider=github", "provider=unknown"),
            valid.replace("provider=github", "provider=github&access_token=private"),
            valid.replace("11111111-1111-4111-8111-111111111111", "not-a-uuid"),
            valid.replace("#confirmation_code=", "&confirmation_code="),
            format!("{valid}&confirmation_code=extra"),
            format!("{valid}\n"),
            "x".repeat(513),
        ] {
            assert!(AuthorizationReturn::parse(&invalid).is_none());
        }
    }
}

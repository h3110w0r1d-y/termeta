use crate::state::{AppState, owner, storage_error};
use tauri::{Manager, State, WebviewWindow};
use terminal_core::{Result, SessionError};

#[tauri::command]
pub async fn get_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>> {
    crate::state::settings_owner(&window)?;
    let Some(setting) = state
        .store
        .get_setting("app")
        .await
        .map_err(storage_error)?
    else {
        return Ok(None);
    };
    if setting.value_version != 1 {
        return Err(SessionError::new(
            "UNSUPPORTED_SETTINGS_VERSION",
            "storage",
            "设置来自不支持的版本，已停止写入",
        ));
    }
    let mut value = setting.value;
    // 旧版自定义字体串收敛到内置默认项，只改本次返回值，保存仍走完整校验。
    if let Some(font) = value.get_mut("fontFamily") {
        if font
            .as_str()
            .is_some_and(|name| !crate::settings::TERMINAL_FONTS.contains(&name))
        {
            *font = serde_json::Value::String(crate::settings::DEFAULT_TERMINAL_FONT.into());
        }
    }
    crate::settings::normalize_legacy_schemes(&mut value);
    crate::settings::validate(&value)?;
    Ok(Some(value))
}
#[tauri::command]
pub async fn save_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
    settings: serde_json::Value,
    defaults: serde_json::Value,
) -> Result<serde_json::Value> {
    use tauri::Emitter;
    crate::state::settings_owner(&window)?;
    // 跨窗口的局部更新在同一临界区内合并，避免整份旧快照覆盖即时偏好。
    let _guard = state.settings_gate.lock().await;
    let current = state
        .store
        .get_setting("app")
        .await
        .map_err(storage_error)?;
    let mut merged = if let Some(current) = current {
        if current.value_version != 1 || current.value["schemaVersion"] != 1 {
            return Err(SessionError::new(
                "UNSUPPORTED_SETTINGS_VERSION",
                "storage",
                "现有设置来自不支持的版本，拒绝覆盖",
            ));
        }
        current.value
    } else {
        crate::settings::validate(&defaults)?;
        defaults
    };
    let patch = settings
        .as_object()
        .ok_or_else(|| SessionError::new("INVALID_CONFIG", "validation", "设置必须为对象"))?;
    for (key, value) in patch {
        // 旧 app 记录可能没有 hostsView，首次局部更新先补齐原有必需字段。
        if key == "hostsView" && merged.get(key).is_none() && value.is_object() {
            merged[key] = serde_json::json!({
                "includeChildren": false,
                "expandedGroupIds": []
            });
        }
        if matches!(key.as_str(), "hostsView" | "hostSearch") {
            if let (Some(target), Some(fields)) = (merged[key].as_object_mut(), value.as_object()) {
                target.extend(fields.clone());
                continue;
            }
        }
        merged[key] = value.clone();
    }
    if merged
        .get("fontFamily")
        .and_then(|value| value.as_str())
        .is_some_and(|name| !crate::settings::TERMINAL_FONTS.contains(&name))
        && !patch.contains_key("fontFamily")
    {
        merged["fontFamily"] =
            serde_json::Value::String(crate::settings::DEFAULT_TERMINAL_FONT.into());
    }
    crate::settings::normalize_legacy_schemes(&mut merged);
    crate::settings::validate(&merged)?;
    state
        .store
        .set_setting("app", 1, &merged)
        .await
        .map_err(storage_error)?;
    // 事件只作为失效通知，接收方重新读取已提交数据。
    let _ = window.app_handle().emit("settings-changed", ());
    Ok(merged)
}

#[tauri::command]
pub fn open_settings_window(window: WebviewWindow) -> Result<()> {
    owner(&window)?;
    crate::settings_window::open(window.app_handle())
        .map_err(|_| SessionError::new("WINDOW_FAILED", "desktop", "无法打开设置窗口"))
}

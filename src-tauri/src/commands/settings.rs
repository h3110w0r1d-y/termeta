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
    crate::settings::normalize_saved(setting.value).map(Some)
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
    let current = if let Some(current) = current {
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
    let merged = crate::settings::apply_patch(current, &settings)?;
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
pub async fn open_settings_window(window: WebviewWindow) -> Result<()> {
    owner(&window)?;
    // Wry/Windows 创建 WebView 必须在独立线程派发；异步 command 避免阻塞事件循环。
    crate::settings_window::open(window.app_handle())
        .map_err(|_| SessionError::new("WINDOW_FAILED", "desktop", "无法打开设置窗口"))
}

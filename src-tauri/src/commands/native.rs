use crate::state::{AppState, owner, storage_error};
#[cfg(target_os = "macos")]
use tauri::Manager;
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use terminal_core::{Result, SessionError};

#[tauri::command]
pub async fn set_native_menu_locale(
    window: WebviewWindow,
    locale: crate::native_menu::NativeMenuLocale,
) -> Result<()> {
    // 锁定页也需跟随系统语言；仅允许本地主窗口更新固定菜单的语言。
    crate::state::window_owner(&window)?;
    #[cfg(target_os = "macos")]
    crate::macos_menu::set_locale(window.app_handle(), locale)
        .await
        .map_err(|_| SessionError::new("MENU_UPDATE_FAILED", "desktop", "无法更新原生菜单语言"))?;
    #[cfg(not(target_os = "macos"))]
    let _ = locale;
    Ok(())
}

#[tauri::command]
pub async fn select_private_key_file(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>> {
    owner(&window)?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("选择 OpenSSH 私钥")
        .pick_file(move |file| {
            let _ = sender.send(file);
        });
    let Some(file) = receiver.await.map_err(|_| {
        SessionError::new("FILE_SELECTION_FAILED", "authentication", "文件选择已取消")
    })?
    else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|_| {
        SessionError::new("INVALID_KEY_FILE", "authentication", "请选择本地私钥文件")
    })?;
    let checked = path.clone();
    tokio::task::spawn_blocking(move || terminal_security::validate_key_file(&checked))
        .await
        .map_err(|_| SessionError::new("INVALID_KEY_FILE", "authentication", "无法检查私钥文件"))?
        .map_err(super::data::security_error)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("私钥")
        .to_owned();
    let path = path.to_str().ok_or_else(|| {
        SessionError::new("INVALID_KEY_FILE", "authentication", "私钥路径编码不受支持")
    })?;
    let key = state
        .store
        .register_key_file(path, &name)
        .await
        .map_err(storage_error)?;
    Ok(Some(serde_json::json!({"keyFileRef":key.id,"name":name})))
}
#[tauri::command]
pub fn open_external_url(window: WebviewWindow, url: String) -> Result<()> {
    owner(&window)?;
    if url.len() > 4096 || url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(invalid_url());
    }
    let parsed = url::Url::parse(&url).map_err(|_| invalid_url())?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(invalid_url());
    }
    window
        .opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|_| SessionError::new("OPEN_URL_FAILED", "desktop", "无法打开系统浏览器"))
}
fn invalid_url() -> SessionError {
    SessionError::new(
        "INVALID_URL",
        "validation",
        "只允许由用户打开有效的 HTTP/HTTPS 链接",
    )
}

#[tauri::command]
pub async fn export_json(
    window: WebviewWindow,
    name: String,
    value: serde_json::Value,
) -> Result<bool> {
    crate::state::settings_owner(&window)?;
    if name.len() > 128
        || !name.ends_with(".json")
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
    {
        return Err(SessionError::new(
            "INVALID_CONFIG",
            "export",
            "导出文件名无效",
        ));
    }
    let bytes = serde_json::to_vec_pretty(&value)
        .map_err(|_| SessionError::new("INVALID_CONFIG", "export", "JSON 数据无效"))?;
    if bytes.len() > 1024 * 1024 {
        return Err(SessionError::new(
            "LIMIT_EXCEEDED",
            "export",
            "导出 JSON 不能超过 1 MiB",
        ));
    }
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("导出 JSON 配置")
        .set_file_name(&name)
        .add_filter("JSON", &["json"])
        .save_file(move |file| {
            let _ = sender.send(file);
        });
    let Some(file) = receiver
        .await
        .map_err(|_| SessionError::new("EXPORT_FAILED", "export", "文件选择器已关闭"))?
    else {
        return Ok(false);
    };
    let path = file
        .into_path()
        .map_err(|_| SessionError::new("EXPORT_FAILED", "export", "请选择本地保存路径"))?;
    // 路径只能来自本次原生保存对话框；普通 IPC 不能提供任意写入路径。
    tokio::fs::write(path, bytes)
        .await
        .map_err(|_| SessionError::new("EXPORT_FAILED", "export", "无法写入选中的文件"))?;
    Ok(true)
}

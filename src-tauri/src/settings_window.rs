use tauri::Manager;

pub fn open(app: &tauri::AppHandle) -> tauri::Result<()> {
    if !app.state::<crate::state::AppState>().vault.is_unlocked() {
        return Ok(());
    }
    if let Some(window) = app.get_webview_window("settings") {
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    tauri::WebviewWindowBuilder::new(app, "settings", tauri::WebviewUrl::App("index.html".into()))
        .title("Termvexa 设置")
        .inner_size(960.0, 720.0)
        .min_inner_size(720.0, 520.0)
        .on_navigation(crate::local_navigation)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .build()?;
    Ok(())
}

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};

#[derive(Default)]
pub struct ShutdownState {
    approved: AtomicBool,
    requested: AtomicBool,
}

pub fn intercept(app: &tauri::AppHandle, api: tauri::ExitRequestApi) {
    let state = app.state::<ShutdownState>();
    if state.approved.load(Ordering::SeqCst) {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        // Cmd+Q、应用菜单和程序退出均先回到主窗口归档，不依赖异步 beforeunload。
        api.prevent_exit();
        state.requested.store(true, Ordering::SeqCst);
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("app-exit-requested", ());
    }
}

#[tauri::command]
pub fn finish_app_exit(window: tauri::WebviewWindow) -> terminal_core::Result<()> {
    // 锁定页也必须能退出；仍只允许受信任的本地主窗口完成退出握手。
    crate::state::window_owner(&window)?;
    let state = window.state::<ShutdownState>();
    if !state.requested.load(Ordering::SeqCst) {
        return Err(terminal_core::SessionError::new(
            "EXIT_NOT_REQUESTED",
            "desktop",
            "应用尚未请求退出",
        ));
    }
    state.approved.store(true, Ordering::SeqCst);
    window.app_handle().exit(0);
    Ok(())
}

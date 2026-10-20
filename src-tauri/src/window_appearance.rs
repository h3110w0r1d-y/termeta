// 与 base.css 及 Naive UI 深色页面底色保持一致。
const BACKGROUND: tauri::utils::config::Color = tauri::utils::config::Color(16, 16, 20, 255);

pub fn prepare<R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'_, R, M>,
) -> tauri::WebviewWindowBuilder<'_, R, M> {
    let builder = builder
        .theme(Some(tauri::Theme::Dark))
        .background_color(BACKGROUND);

    #[cfg(target_os = "windows")]
    let builder = {
        use std::sync::atomic::{AtomicBool, Ordering};

        let shown = AtomicBool::new(false);
        // Windows 先创建 HWND 再初始化 WebView2；仅设置深色背景仍会暴露创建阶段的画面。
        builder.visible(false).on_page_load(move |window, payload| {
            if payload.event() != tauri::webview::PageLoadEvent::Finished
                || !crate::local_navigation(payload.url())
                || shown.swap(true, Ordering::Relaxed)
            {
                return;
            }
            // 仅首次加载负责显示，后续刷新不能恢复已最小化的窗口或抢走焦点。
            if let Err(error) = window.show().and_then(|()| window.set_focus()) {
                eprintln!("无法显示已加载的窗口 {}: {error}", window.label());
            }
        })
    };

    builder
}

use std::sync::atomic::{AtomicBool, Ordering};

// 与入口 HTML、base.css 及 Naive UI 深色页面底色保持一致。
const BACKGROUND: tauri::utils::config::Color = tauri::utils::config::Color(16, 16, 20, 255);

pub fn prepare<R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'_, R, M>,
) -> tauri::WebviewWindowBuilder<'_, R, M> {
    let shown = AtomicBool::new(false);
    builder
        .theme(Some(tauri::Theme::Dark))
        .background_color(BACKGROUND)
        .visible(false)
        .on_page_load(move |window, payload| {
            // macOS 的 WebView 背景 API 不生效，首次本地文档加载完成后才展示。
            // 只处理首次加载，避免刷新页面时重新弹出或抢占窗口焦点。
            if payload.event() != tauri::webview::PageLoadEvent::Finished
                || !crate::local_navigation(payload.url())
                || shown.swap(true, Ordering::Relaxed)
            {
                return;
            }
            if let Err(error) = window.show().and_then(|()| window.set_focus()) {
                eprintln!("显示窗口失败：{error}");
            }
        })
}

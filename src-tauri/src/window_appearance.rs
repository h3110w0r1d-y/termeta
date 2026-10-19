// 与 base.css 及 Naive UI 深色页面底色保持一致。
const BACKGROUND: tauri::utils::config::Color = tauri::utils::config::Color(16, 16, 20, 255);

pub fn prepare<R: tauri::Runtime, M: tauri::Manager<R>>(
    builder: tauri::WebviewWindowBuilder<'_, R, M>,
) -> tauri::WebviewWindowBuilder<'_, R, M> {
    builder
        .theme(Some(tauri::Theme::Dark))
        .background_color(BACKGROUND)
}

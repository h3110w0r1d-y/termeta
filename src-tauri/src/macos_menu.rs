use tauri::{
    Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu, WINDOW_SUBMENU_ID},
};

pub fn configure(app: &tauri::App) -> tauri::Result<()> {
    let menu = Menu::default(app.handle())?;
    // 系统预设Close Window自带Cmd+W，会绕过WebView的弹窗/标签路由。
    // 保留窗口菜单操作，但用无快捷键的普通项替代，让Cmd+W只由前端处理。
    let window_menu = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "termeta.closeWindow",
                "Close Window",
                true,
                None::<&str>,
            )?,
        ],
    )?;
    for (index, item) in menu.items()?.into_iter().enumerate() {
        if item.id().as_ref() == WINDOW_SUBMENU_ID {
            menu.remove(&item)?;
            menu.insert(&window_menu, index)?;
            break;
        }
    }
    if let Some(tauri::menu::MenuItemKind::Submenu(application)) = menu.items()?.first() {
        application.insert(
            &MenuItem::with_id(app, "termeta.settings", "设置…", true, None::<&str>)?,
            1,
        )?;
    }
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        if event.id().as_ref() == "termeta.settings" {
            let _ = crate::settings_window::open(app);
        }
        if event.id().as_ref() == "termeta.closeWindow" {
            if let Some(window) = app
                .webview_windows()
                .into_values()
                .find(|window| window.is_focused().unwrap_or(false))
            {
                // 显式菜单关闭仍走现有CloseRequested与会话/传输确认流程。
                let _ = window.close();
            }
        }
    });
    Ok(())
}

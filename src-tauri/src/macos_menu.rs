use block2::RcBlock;
use objc2::{rc::Retained, runtime::AnyObject};
use objc2_app_kit::{NSApplication, NSEvent, NSEventMask, NSEventModifierFlags};
use objc2_foundation::MainThreadMarker;
use std::{cell::RefCell, ptr::NonNull};
use tauri::{
    Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu, WINDOW_SUBMENU_ID},
};

struct SettingsShortcutMonitor(Retained<AnyObject>);

impl Drop for SettingsShortcutMonitor {
    fn drop(&mut self) {
        // 令牌仅由主线程持有，移除后 AppKit 不再调用对应的事件块。
        unsafe { NSEvent::removeMonitor(&self.0) };
    }
}

thread_local! {
    static SETTINGS_SHORTCUT_MONITOR: RefCell<Option<SettingsShortcutMonitor>> = const { RefCell::new(None) };
}

fn install_settings_shortcut_monitor() -> tauri::Result<()> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| std::io::Error::other("设置快捷键必须在主线程安装"))?;
    let handler = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
        // AppKit 在此同步借用原事件；只返回该有效指针或空指针，不跨回调保存事件。
        let native_event = unsafe { event.as_ref() };
        let shortcut_modifiers = NSEventModifierFlags::Command
            | NSEventModifierFlags::Control
            | NSEventModifierFlags::Option
            | NSEventModifierFlags::Shift;
        if native_event.modifierFlags() & shortcut_modifiers != NSEventModifierFlags::Command
            || !native_event
                .charactersIgnoringModifiers()
                .is_some_and(|characters| characters.to_string() == ",")
        {
            return event.as_ptr();
        }
        let application = NSApplication::sharedApplication(mtm);
        if !application.isActive() || application.keyWindow().is_none() {
            return event.as_ptr();
        }
        // 在 WebKit 的输入框接收按键之前交给现有原生菜单，避免依赖响应链继续冒泡。
        // 菜单成功处理后返回空指针，阻止同一按键再次进入 WebView 或重复触发菜单。
        if application
            .mainMenu()
            .is_some_and(|menu| menu.performKeyEquivalent(native_event))
        {
            std::ptr::null_mut()
        } else {
            event.as_ptr()
        }
    });
    // 回调返回值始终为传入的有效事件或空指针，满足 AppKit 的事件监视器契约。
    let monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, &handler)
    }
    .ok_or_else(|| std::io::Error::other("无法安装设置快捷键事件监视器"))?;
    SETTINGS_SHORTCUT_MONITOR.with_borrow_mut(|slot| {
        *slot = Some(SettingsShortcutMonitor(monitor));
    });
    Ok(())
}

pub fn remove_settings_shortcut_monitor() {
    // 调用方通过主线程调度，主窗口销毁时同时移除应用内监听。
    SETTINGS_SHORTCUT_MONITOR.with_borrow_mut(|slot| {
        drop(slot.take());
    });
}

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
            &MenuItem::with_id(app, "termeta.settings", "设置", true, Some("Cmd+,"))?,
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
    install_settings_shortcut_monitor()
}

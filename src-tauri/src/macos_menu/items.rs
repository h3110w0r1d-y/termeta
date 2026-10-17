use crate::native_menu::NativeMenuLocale;
use std::collections::HashMap;
use tauri::{
    Manager, Wry,
    menu::{
        AboutMetadata, HELP_SUBMENU_ID, IsMenuItem, Menu, MenuItem, MenuItemKind,
        PredefinedMenuItem, Submenu, WINDOW_SUBMENU_ID,
    },
};

type LabelledItem = (&'static str, MenuItemKind<Wry>);

pub(super) struct LocalizedMenu {
    items: Vec<LabelledItem>,
}

fn text<'a>(labels: &'a HashMap<String, String>, key: &str) -> tauri::Result<&'a str> {
    labels
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| std::io::Error::other(format!("原生菜单缺少词条：{key}")).into())
}

fn track<T: IsMenuItem<Wry>>(items: &mut Vec<LabelledItem>, key: &'static str, item: T) -> T {
    items.push((key, item.kind()));
    item
}

impl LocalizedMenu {
    pub(super) fn install(app: &tauri::App) -> tauri::Result<()> {
        // WebView 就绪前采用英文启动文案，随后由主窗口同步系统或已保存的语言。
        let labels = NativeMenuLocale::EnUs.labels()?;
        let mut items = Vec::new();
        macro_rules! predefined {
            ($method:ident, $key:literal) => {
                track(
                    &mut items,
                    $key,
                    PredefinedMenuItem::$method(app, Some(text(&labels, $key)?))?,
                )
            };
        }
        let about = track(
            &mut items,
            "nativeMenu.app.about",
            PredefinedMenuItem::about(
                app,
                Some(text(&labels, "nativeMenu.app.about")?),
                Some(AboutMetadata {
                    name: Some("Termeta".into()),
                    version: Some(app.package_info().version.to_string()),
                    copyright: app.config().bundle.copyright.clone(),
                    authors: app.config().bundle.publisher.clone().map(|name| vec![name]),
                    ..Default::default()
                }),
            )?,
        );
        let settings = track(
            &mut items,
            "nativeMenu.app.settings",
            MenuItem::with_id(
                app,
                "termeta.settings",
                text(&labels, "nativeMenu.app.settings")?,
                true,
                Some("Cmd+,"),
            )?,
        );
        let services = predefined!(services, "nativeMenu.app.services");
        let hide = predefined!(hide, "nativeMenu.app.hide");
        let hide_others = predefined!(hide_others, "nativeMenu.app.hideOthers");
        let show_all = predefined!(show_all, "nativeMenu.app.showAll");
        let quit = predefined!(quit, "nativeMenu.app.quit");
        let application = Submenu::with_items(
            app,
            "Termeta",
            true,
            &[
                &about,
                &settings,
                &PredefinedMenuItem::separator(app)?,
                &services,
                &PredefinedMenuItem::separator(app)?,
                &hide,
                &hide_others,
                &show_all,
                &PredefinedMenuItem::separator(app)?,
                &quit,
            ],
        )?;
        // 文件与窗口菜单共用无快捷键的关闭项，Cmd+W 继续由弹窗和标签路由处理。
        let close = track(
            &mut items,
            "nativeMenu.window.close",
            MenuItem::with_id(
                app,
                "termeta.closeWindow",
                text(&labels, "nativeMenu.window.close")?,
                true,
                None::<&str>,
            )?,
        );
        let file = track(
            &mut items,
            "nativeMenu.file.title",
            Submenu::with_items(
                app,
                text(&labels, "nativeMenu.file.title")?,
                true,
                &[&close],
            )?,
        );
        let undo = predefined!(undo, "nativeMenu.edit.undo");
        let redo = predefined!(redo, "nativeMenu.edit.redo");
        let cut = predefined!(cut, "nativeMenu.edit.cut");
        let copy = predefined!(copy, "nativeMenu.edit.copy");
        let paste = predefined!(paste, "nativeMenu.edit.paste");
        let select_all = predefined!(select_all, "nativeMenu.edit.selectAll");
        let edit = track(
            &mut items,
            "nativeMenu.edit.title",
            Submenu::with_items(
                app,
                text(&labels, "nativeMenu.edit.title")?,
                true,
                &[
                    &undo,
                    &redo,
                    &PredefinedMenuItem::separator(app)?,
                    &cut,
                    &copy,
                    &paste,
                    &select_all,
                ],
            )?,
        );
        let fullscreen = predefined!(fullscreen, "nativeMenu.view.fullscreen");
        let view = track(
            &mut items,
            "nativeMenu.view.title",
            Submenu::with_items(
                app,
                text(&labels, "nativeMenu.view.title")?,
                true,
                &[&fullscreen],
            )?,
        );
        let minimize = predefined!(minimize, "nativeMenu.window.minimize");
        let maximize = predefined!(maximize, "nativeMenu.window.maximize");
        let window = track(
            &mut items,
            "nativeMenu.window.title",
            Submenu::with_id_and_items(
                app,
                WINDOW_SUBMENU_ID,
                text(&labels, "nativeMenu.window.title")?,
                true,
                &[
                    &minimize,
                    &maximize,
                    &PredefinedMenuItem::separator(app)?,
                    &close,
                ],
            )?,
        );
        let help = track(
            &mut items,
            "nativeMenu.help.title",
            Submenu::with_id_and_items(
                app,
                HELP_SUBMENU_ID,
                text(&labels, "nativeMenu.help.title")?,
                true,
                &[],
            )?,
        );
        let menu = Menu::with_items(app, &[&application, &file, &edit, &view, &window, &help])?;
        app.set_menu(menu)?;
        app.manage(Self { items });
        Ok(())
    }

    pub(super) fn set_locale(&self, locale: NativeMenuLocale) -> tauri::Result<()> {
        let labels = locale.labels()?;
        // 先核对完整资源，再原位更新所有标题；不替换菜单对象、快捷键或事件处理器。
        let updates = self
            .items
            .iter()
            .map(|(key, item)| Ok((item, text(&labels, key)?)))
            .collect::<tauri::Result<Vec<_>>>()?;
        for (item, label) in updates {
            match item {
                MenuItemKind::MenuItem(item) => item.set_text(label)?,
                MenuItemKind::Submenu(item) => item.set_text(label)?,
                MenuItemKind::Predefined(item) => item.set_text(label)?,
                MenuItemKind::Check(item) => item.set_text(label)?,
                MenuItemKind::Icon(item) => item.set_text(label)?,
            }
        }
        Ok(())
    }
}

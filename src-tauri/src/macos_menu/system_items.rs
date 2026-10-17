use crate::native_menu::NativeMenuLocale;
use block2::RcBlock;
use objc2::{ClassType, rc::Retained, runtime::ProtocolObject, sel};
use objc2_app_kit::{
    NSApplication, NSMenu, NSMenuDidAddItemNotification, NSMenuDidBeginTrackingNotification,
    NSMenuDidChangeItemNotification, NSMenuItem, NSWindow,
};
use objc2_foundation::{
    MainThreadMarker, NSBundle, NSNotification, NSNotificationCenter, NSObjectProtocol, NSString,
};
use std::{
    cell::{Cell, RefCell},
    ptr::NonNull,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Edit,
    AutoFill,
    Window,
    MoveResize,
    FullScreenTile,
}

struct Rule {
    scope: Scope,
    key: &'static str,
    table: &'static str,
    system_key: &'static str,
    children: Option<Scope>,
}

macro_rules! rule {
    ($scope:ident, $key:literal, $table:literal, $system_key:literal) => {
        Rule {
            scope: Scope::$scope,
            key: $key,
            table: $table,
            system_key: $system_key,
            children: None,
        }
    };
    ($scope:ident, $key:literal, $table:literal, $system_key:literal, $children:ident) => {
        Rule {
            children: Some(Scope::$children),
            ..rule!($scope, $key, $table, $system_key)
        }
    };
}

// AppKit 没有为这些动态项公开统一 ID；只在指定菜单层级内匹配固定标签。
// 表名/key 来自系统资源，查找失败仍可识别应用支持的中英文，不调用私有动作。
const RULES: &[Rule] = &[
    rule!(
        Edit,
        "nativeMenu.edit.autoFill",
        "InputManager",
        "AutoFill",
        AutoFill
    ),
    rule!(
        Edit,
        "nativeMenu.edit.startDictation",
        "DictationManager",
        "Start Dictation…"
    ),
    rule!(
        Edit,
        "nativeMenu.edit.stopDictation",
        "DictationManager",
        "Stop Dictation"
    ),
    rule!(
        Edit,
        "nativeMenu.edit.emojiSymbols",
        "InputManager",
        "Emoji & Symbols"
    ),
    rule!(
        AutoFill,
        "nativeMenu.edit.autoFillContact",
        "InputManager",
        "Contact…"
    ),
    rule!(
        AutoFill,
        "nativeMenu.edit.autoFillPasswords",
        "InputManager",
        "Passwords…"
    ),
    rule!(
        AutoFill,
        "nativeMenu.edit.autoFillPasswordsAndCodes",
        "InputManager",
        "Passwords & Verification Codes…"
    ),
    rule!(Window, "nativeMenu.window.fill", "MenuCommands", "Fill"),
    rule!(Window, "nativeMenu.window.center", "MenuCommands", "Center"),
    rule!(
        Window,
        "nativeMenu.window.removeFromSet",
        "MenuCommands",
        "Remove Window from Set"
    ),
    rule!(
        Window,
        "nativeMenu.window.moveResize",
        "MenuCommands",
        "Move & Resize",
        MoveResize
    ),
    rule!(
        Window,
        "nativeMenu.window.fullScreenTile",
        "MenuCommands",
        "Full Screen Tile",
        FullScreenTile
    ),
    rule!(
        Window,
        "nativeMenu.window.enterFullScreen",
        "MenuCommands",
        "Enter Full Screen"
    ),
    rule!(
        Window,
        "nativeMenu.window.exitFullScreen",
        "MenuCommands",
        "Exit Full Screen"
    ),
    rule!(
        Window,
        "nativeMenu.window.bringAllToFront",
        "MenuCommands",
        "Bring All to Front"
    ),
    rule!(
        Window,
        "nativeMenu.window.replaceTiledWindow",
        "MenuCommands",
        "Replace Tiled Window"
    ),
    rule!(
        Window,
        "nativeMenu.window.tileLeft",
        "MenuCommands",
        "Tile Window to Left of Screen"
    ),
    rule!(
        Window,
        "nativeMenu.window.tileRight",
        "MenuCommands",
        "Tile Window to Right of Screen"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.halves",
        "MenuCommands",
        "Halves"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.left",
        "MenuCommands",
        "Left"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.right",
        "MenuCommands",
        "Right"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.top",
        "MenuCommands",
        "Top"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.bottom",
        "MenuCommands",
        "Bottom"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.quarters",
        "MenuCommands",
        "Quarters_Header"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.topLeft",
        "MenuCommands",
        "Top Left"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.topRight",
        "MenuCommands",
        "Top Right"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.bottomLeft",
        "MenuCommands",
        "Bottom Left"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.bottomRight",
        "MenuCommands",
        "Bottom Right"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.arrange",
        "MenuCommands",
        "Arrange"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.fillArrange",
        "MenuCommands",
        "Fill & Arrange"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.leftRight",
        "MenuCommands",
        "Left & Right"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.rightLeft",
        "MenuCommands",
        "Right & Left"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.topBottom",
        "MenuCommands",
        "Top & Bottom"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.bottomTop",
        "MenuCommands",
        "Bottom & Top"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.leftQuarters",
        "MenuCommands",
        "Left & Quarters"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.rightQuarters",
        "MenuCommands",
        "Right & Quarters"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.topQuarters",
        "MenuCommands",
        "Top & Quarters"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.bottomQuarters",
        "MenuCommands",
        "Bottom & Quarters"
    ),
    rule!(
        MoveResize,
        "nativeMenu.window.layout.previousSize",
        "MenuCommands",
        "Return to Previous Size"
    ),
    rule!(
        FullScreenTile,
        "nativeMenu.window.tile.left",
        "MenuCommands",
        "Left of Screen"
    ),
    rule!(
        FullScreenTile,
        "nativeMenu.window.tile.right",
        "MenuCommands",
        "Right of Screen"
    ),
    rule!(
        FullScreenTile,
        "nativeMenu.window.tile.entire",
        "MenuCommands",
        "Entire Screen"
    ),
];

struct Translation {
    rule: &'static Rule,
    english: String,
    chinese: String,
    aliases: Vec<String>,
}

impl Translation {
    fn label(&self, locale: NativeMenuLocale) -> &str {
        match locale {
            NativeMenuLocale::ZhCn => &self.chinese,
            NativeMenuLocale::EnUs => &self.english,
        }
    }
}

struct MenuObserver(Retained<ProtocolObject<dyn NSObjectProtocol>>);

impl Drop for MenuObserver {
    fn drop(&mut self) {
        // 令牌仅在主线程存取，先注销再释放，避免主窗口销毁后的迟到回调。
        unsafe { NSNotificationCenter::defaultCenter().removeObserver((*self.0).as_ref()) };
    }
}

struct SystemMenuLocalizer {
    edit: Retained<NSMenu>,
    window: Retained<NSMenu>,
    translations: Vec<Translation>,
    locale: NativeMenuLocale,
    observers: Vec<MenuObserver>,
}

thread_local! {
    static LOCALIZER: RefCell<Option<SystemMenuLocalizer>> = const { RefCell::new(None) };
    static REFRESHING: Cell<bool> = const { Cell::new(false) };
}

struct RefreshGuard;

impl Drop for RefreshGuard {
    fn drop(&mut self) {
        REFRESHING.set(false);
    }
}

impl SystemMenuLocalizer {
    fn translate(&self, menu: &NSMenu, scope: Scope) {
        // 使用快照遍历；只改标题，不增删条目，不调用 update 或替换系统 delegate。
        for item in menu.itemArray().to_vec() {
            if item.isSeparatorItem() || is_window_entry(&item) {
                continue;
            }
            let title = item.title().to_string();
            // 不同系统版本可能使用三个句点或单个省略号。
            let normalized = title.replace("...", "…");
            let Some(translation) = self.translations.iter().find(|translation| {
                translation.rule.scope == scope && translation.aliases.contains(&normalized)
            }) else {
                continue;
            };
            let label = translation.label(self.locale);
            if title != label {
                item.setTitle(&NSString::from_str(label));
            }
            // 仅进入已知系统子菜单，保留服务、窗口名和自动填充建议等动态内容。
            if let Some((submenu, children)) = item.submenu().zip(translation.rule.children) {
                if submenu.title().to_string() != label {
                    submenu.setTitle(&NSString::from_str(label));
                }
                self.translate(&submenu, children);
            }
        }
    }
}

fn is_window_entry(item: &NSMenuItem) -> bool {
    // 窗口列表可能与固定动作重名，必须保留其标题及任何关联的用户内容。
    item.action() == Some(sel!(makeKeyAndOrderFront:))
        || item
            .representedObject()
            .is_some_and(|object| object.downcast_ref::<NSWindow>().is_some())
}

fn refresh() {
    // 通知不指定队列，按 AppKit 的原始同步顺序处理；非主线程的通知直接忽略。
    if MainThreadMarker::new().is_none() || REFRESHING.replace(true) {
        return;
    }
    let _guard = RefreshGuard;
    LOCALIZER.with_borrow(|slot| {
        if let Some(localizer) = slot {
            localizer.translate(&localizer.edit, Scope::Edit);
            localizer.translate(&localizer.window, Scope::Window);
        }
    });
}

pub(super) fn install() -> tauri::Result<()> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| std::io::Error::other("系统菜单翻译必须在主线程安装"))?;
    let application = NSApplication::sharedApplication(mtm);
    // 通过原生编辑动作定位，不依赖菜单的当前语言或顶层排列顺序。
    let edit = application.mainMenu().and_then(|main| {
        main.itemArray().to_vec().into_iter().find_map(|item| {
            item.submenu().filter(|menu| {
                menu.itemArray()
                    .to_vec()
                    .into_iter()
                    .any(|item| item.action() == Some(sel!(selectAll:)))
            })
        })
    });
    let edit = edit.ok_or_else(|| std::io::Error::other("未找到原生编辑菜单"))?;
    let window = application
        .windowsMenu()
        .ok_or_else(|| std::io::Error::other("未找到原生窗口菜单"))?;
    let english = NativeMenuLocale::EnUs.labels()?;
    let chinese = NativeMenuLocale::ZhCn.labels()?;
    // NSMenu 是有效的 AppKit 类，只读取框架按系统语言选出的固定标签作为识别别名。
    let bundle = unsafe { NSBundle::bundleForClass(NSMenu::class()) };
    let translations = RULES
        .iter()
        .map(|rule| {
            let missing = || std::io::Error::other(format!("系统菜单缺少词条：{}", rule.key));
            let english = english.get(rule.key).ok_or_else(missing)?.clone();
            let chinese = chinese.get(rule.key).ok_or_else(missing)?.clone();
            let system = bundle
                .localizedStringForKey_value_table(
                    &NSString::from_str(rule.system_key),
                    None,
                    Some(&NSString::from_str(rule.table)),
                )
                .to_string();
            let aliases = [&english, &chinese, &system]
                .into_iter()
                .map(|title| title.replace("...", "…"))
                .collect();
            Ok(Translation {
                rule,
                english,
                chinese,
                aliases,
            })
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    LOCALIZER.with_borrow_mut(|slot| {
        *slot = Some(SystemMenuLocalizer {
            edit,
            window,
            translations,
            locale: NativeMenuLocale::EnUs,
            observers: Vec::new(),
        });
    });
    let center = NSNotificationCenter::defaultCenter();
    let handler = RcBlock::new(|_: NonNull<NSNotification>| refresh());
    // 新增、系统重设标题和开始菜单跟踪都需同步适配；保留 muda 与系统的 delegate。
    // 回调不捕获主线程对象，且 refresh 自行检查线程，满足通知块的可发送要求。
    let observers = unsafe {
        [
            NSMenuDidAddItemNotification,
            NSMenuDidChangeItemNotification,
            NSMenuDidBeginTrackingNotification,
        ]
        .into_iter()
        .map(|name| {
            MenuObserver(center.addObserverForName_object_queue_usingBlock(
                Some(name),
                None,
                None,
                &handler,
            ))
        })
        .collect()
    };
    LOCALIZER.with_borrow_mut(|slot| {
        if let Some(localizer) = slot {
            localizer.observers = observers;
        }
    });
    refresh();
    Ok(())
}

pub(super) fn set_locale(locale: NativeMenuLocale) {
    LOCALIZER.with_borrow_mut(|slot| {
        if let Some(localizer) = slot {
            localizer.locale = locale;
        }
    });
    refresh();
}

pub(super) fn remove() {
    LOCALIZER.with_borrow_mut(|slot| {
        drop(slot.take());
    });
}

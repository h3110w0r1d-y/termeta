pub const TERMINAL_FONTS: &[&str] = &[
    "DejaVu Sans Mono",
    "Droid Sans Mono",
    "Ubuntu Mono",
    "Anonymous Pro",
    "Inconsolata",
    "Fira Mono",
    "Source Code Pro",
    "PT Mono",
    "Cascadia Code",
    "Fira Code",
    "JetBrains Mono",
    "Meslo",
];
pub const DEFAULT_TERMINAL_FONT: &str = "DejaVu Sans Mono";

use serde_json::Value;
use terminal_core::{Result, SessionError};

fn invalid() -> SessionError {
    SessionError::new("INVALID_SETTINGS", "validation", "设置格式、取值或引用无效")
}
fn color(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        (text.len() == 7 || text.len() == 9)
            && text.starts_with('#')
            && text[1..].bytes().all(|c| c.is_ascii_hexdigit())
    })
}

// 设置只允许声明式数据，不接受 CSS 表达式、任意原生动作或脚本。
pub fn validate(settings: &Value) -> Result<()> {
    if serde_json::to_vec(settings).map_err(|_| invalid())?.len() > 256 * 1024 {
        return Err(invalid());
    }
    let object = settings.as_object().ok_or_else(invalid)?;
    let allowed = [
        "schemaVersion",
        "appearance",
        "language",
        "primaryColor",
        "tabPosition",
        "tabDoubleClickAction",
        "sftpShowHidden",
        "sftpPosition",
        "detectSshSystem",
        "hostsView",
        "hostSearch",
        "fontFamily",
        "fontSize",
        "scrollback",
        "defaultSchemeId",
        "confirmClose",
        "multilinePaste",
        "terminalRightClickAction",
        "terminalMiddleClickAction",
        "keymap",
        "customSchemes",
        "hostSchemes",
    ];
    if object.keys().any(|key| !allowed.contains(&key.as_str()))
        || settings["schemaVersion"] != 1
        || !matches!(
            settings["appearance"].as_str(),
            Some("system" | "dark" | "light")
        )
        // 旧设置缺失时跟随系统；拒绝不受支持的语言及非字符串值。
        || object.get("language").is_some_and(|value| {
            !matches!(value.as_str(), Some("system" | "zh-CN" | "zh-TW" | "en-US" | "ja-JP" | "ko-KR"))
        })
        // 缺失或空串沿用库默认主题色；自定义值只接受不透明的六位 HEX。
        || object.get("primaryColor").is_some_and(|value| {
            !value.as_str().is_some_and(|text| {
                text.is_empty() || (text.len() == 7 && color(value))
            })
        })
        // 兼容尚未保存布局偏好的旧设置；新值只允许顶部或左侧标签栏。
        || object.get("tabPosition").is_some_and(|value| {
            !matches!(value.as_str(), Some("top" | "left"))
        })
        // 旧设置缺失时双击无动作；仅允许声明式的内置标签动作。
        || object.get("tabDoubleClickAction").is_some_and(|value| {
            !matches!(value.as_str(), Some("none" | "close" | "duplicate" | "split"))
        })
        // 缺少位置偏好的旧设置继续显示在右侧。
        || object.get("sftpPosition").is_some_and(|value| {
            !matches!(value.as_str(), Some("left" | "right"))
        })
        // 旧设置缺失时由前端默认隐藏；已保存的值必须是布尔值。
        || object.get("sftpShowHidden").is_some_and(|value| !value.is_boolean())
        || object.get("detectSshSystem").is_some_and(|value| !value.is_boolean())
        // 旧设置缺失时不执行本地鼠标动作；仅允许内置的粘贴行为。
        || ["terminalRightClickAction", "terminalMiddleClickAction"].iter().any(|key| {
            object.get(*key).is_some_and(|value| {
                !matches!(value.as_str(), Some("none" | "paste"))
            })
        })
        || !settings["fontSize"]
            .as_f64()
            .is_some_and(|n| (10.0..=32.0).contains(&n))
        || !settings["scrollback"]
            .as_u64()
            .is_some_and(|n| (1000..=50000).contains(&n))
        || !settings["fontFamily"]
            .as_str()
            .is_some_and(|s| TERMINAL_FONTS.contains(&s))
        || !settings["confirmClose"].is_boolean()
        || !settings["multilinePaste"].is_boolean()
    {
        return Err(invalid());
    }
    // 主机页即时偏好兼容旧字段缺失；限制开关、显示模式和有界分组 UUID 集合。
    if let Some(view) = object.get("hostsView") {
        fields(
            view,
            &[
                "includeChildren",
                "expandedGroupIds",
                "showHosts",
                "displayMode",
            ],
        )?;
        let expanded = view["expandedGroupIds"].as_array().ok_or_else(invalid)?;
        let mut ids = std::collections::HashSet::new();
        if view
            .get("showHosts")
            .is_some_and(|value| !value.is_boolean())
            || view
                .get("displayMode")
                .is_some_and(|value| !matches!(value.as_str(), Some("card" | "list")))
            || !view["includeChildren"].is_boolean()
            || expanded.len() > 10000
            || expanded.iter().any(|value| {
                !value.as_str().is_some_and(|id| {
                    (id == "ungrouped" || uuid::Uuid::parse_str(id).is_ok()) && ids.insert(id)
                })
            })
        {
            return Err(invalid());
        }
    }
    // 旧设置缺失时沿用全部字段和拼音；名称始终可搜索，字段不得重复。
    if let Some(search) = object.get("hostSearch") {
        fields(search, &["pinyin", "fields"])?;
        let selected = search["fields"].as_array().ok_or_else(invalid)?;
        let mut unique = std::collections::HashSet::new();
        if !search["pinyin"].is_boolean()
            || !selected.iter().any(|field| field == "name")
            || selected.iter().any(|field| {
                !field.as_str().is_some_and(|key| {
                    matches!(key, "name" | "address" | "username" | "tags" | "group")
                        && unique.insert(key)
                })
            })
        {
            return Err(invalid());
        }
    }
    let schemes = settings["customSchemes"].as_array().ok_or_else(invalid)?;
    if schemes.len() > 64 {
        return Err(invalid());
    }
    let mut ids = std::collections::HashSet::new();
    for scheme in schemes {
        fields(scheme, &["schemaVersion", "id", "name", "source", "colors"])?;
        let id = scheme["id"].as_str().ok_or_else(invalid)?;
        if scheme["schemaVersion"] != 1
            || id.is_empty()
            || builtin(id)
            || id.len() > 128
            || !ids.insert(id)
            || !scheme["name"].as_str().is_some_and(|n| {
                !n.trim().is_empty() && n.len() <= 128 && !n.chars().any(char::is_control)
            })
            || !matches!(scheme["source"].as_str(), Some("user" | "imported"))
        {
            return Err(invalid());
        }
        let colors = scheme["colors"].as_object().ok_or_else(invalid)?;
        for field in [
            "foreground",
            "background",
            "cursor",
            "cursorAccent",
            "selectionBackground",
            "selectionInactiveBackground",
        ] {
            if !color(colors.get(field).ok_or_else(invalid)?) {
                return Err(invalid());
            }
        }
        // UI 辅助色为可选扩展，兼容未保存该字段的旧配色。
        if let Some(value) = colors.get("ui") {
            if !color(value) {
                return Err(invalid());
            }
        }
        if let Some(value) = colors.get("selectionForeground") {
            if !color(value) {
                return Err(invalid());
            }
        }
        let ansi = colors
            .get("ansi")
            .and_then(Value::as_array)
            .ok_or_else(invalid)?;
        if ansi.len() != 16 || !ansi.iter().all(color) {
            return Err(invalid());
        }
        if let Some(indexed) = colors.get("indexedOverrides") {
            let indexed = indexed.as_object().ok_or_else(invalid)?;
            if indexed.len() > 240
                || indexed
                    .iter()
                    .any(|(k, v)| !k.parse::<u8>().is_ok_and(|n| n >= 16) || !color(v))
            {
                return Err(invalid());
            }
        }
        if let Some(scrollbar) = colors.get("scrollbar") {
            let scrollbar = scrollbar.as_object().ok_or_else(invalid)?;
            if scrollbar
                .iter()
                .any(|(k, v)| !["normal", "hover", "active"].contains(&k.as_str()) || !color(v))
            {
                return Err(invalid());
            }
        }
        if colors.keys().any(|k| {
            ![
                "foreground",
                "background",
                "ui",
                "cursor",
                "cursorAccent",
                "selectionBackground",
                "selectionForeground",
                "selectionInactiveBackground",
                "ansi",
                "indexedOverrides",
                "scrollbar",
            ]
            .contains(&k.as_str())
        }) {
            return Err(invalid());
        }
    }
    let default = settings["defaultSchemeId"].as_str().ok_or_else(invalid)?;
    if !(builtin(default) || ids.contains(default)) {
        return Err(invalid());
    }
    let overrides = settings["hostSchemes"].as_object().ok_or_else(invalid)?;
    if overrides.len() > 10000
        || overrides.iter().any(|(host, scheme)| {
            uuid::Uuid::parse_str(host).is_err()
                || !scheme
                    .as_str()
                    .is_some_and(|id| builtin(id) || ids.contains(id))
        })
    {
        return Err(invalid());
    }
    let keymap = &settings["keymap"];
    fields(keymap, &["schemaVersion", "bindings"])?;
    let bindings = keymap["bindings"].as_array().ok_or_else(invalid)?;
    if keymap["schemaVersion"] != 2 || bindings.len() > 128 {
        return Err(invalid());
    }
    let mut binding_ids = std::collections::HashSet::new();
    for binding in bindings {
        fields(
            binding,
            &[
                "id",
                "commandId",
                "scope",
                "context",
                "trigger",
                "platform",
                "enabled",
            ],
        )?;
        let id = binding["id"].as_str().ok_or_else(invalid)?;
        if id.is_empty()
            || id.len() > 128
            || !binding_ids.insert(id)
            || !binding["enabled"].is_boolean()
            || binding["platform"] != "all"
            || !matches!(binding["scope"].as_str(), Some("application" | "terminal"))
            || !matches!(binding["context"].as_str(), Some("workbench" | "terminal"))
            || !command(binding["commandId"].as_str().ok_or_else(invalid)?)
        {
            return Err(invalid());
        }
        let terminal = binding["commandId"]
            .as_str()
            .is_some_and(|id| id.starts_with("terminal."));
        if binding["scope"] != if terminal { "terminal" } else { "application" }
            || binding["context"] != if terminal { "terminal" } else { "workbench" }
        {
            return Err(invalid());
        }
        let trigger = &binding["trigger"];
        let modifiers = trigger["modifiers"].as_array().ok_or_else(invalid)?;
        if modifiers.len() > 5
            || modifiers.iter().any(|m| {
                !matches!(
                    m.as_str(),
                    Some("Primary" | "Control" | "Meta" | "Alt" | "Shift")
                )
            })
        {
            return Err(invalid());
        }
        fields(
            trigger,
            if trigger["type"] == "key" {
                &["type", "key", "modifiers"]
            } else {
                &["type", "button", "event", "modifiers"]
            },
        )?;
        match trigger["type"].as_str() {
            Some("key")
                if trigger["key"].as_str().is_some_and(|k| {
                    (!k.is_empty() || binding["enabled"] == false)
                        && k.len() <= 32
                        && !k.chars().any(char::is_control)
                }) => {}
            Some("pointer")
                if trigger["button"] == "primary"
                    && trigger["event"] == "click"
                    && binding["commandId"] == "terminal.openLink" => {}
            _ => return Err(invalid()),
        }
    }
    Ok(())
}
#[derive(serde::Deserialize)]
struct BuiltinPaletteId {
    id: String,
}
fn builtin(id: &str) -> bool {
    // 与前端读取同一静态资源，避免新增配色后遗漏后端白名单。
    static PALETTES: std::sync::OnceLock<Vec<BuiltinPaletteId>> = std::sync::OnceLock::new();
    PALETTES
        .get_or_init(|| {
            serde_json::from_str(include_str!("../../resources/terminal-palettes.json"))
                .expect("内置配色资源必须为有效 JSON")
        })
        .iter()
        .any(|palette| palette.id == id)
}
pub fn normalize_legacy_schemes(settings: &mut Value) {
    fn replacement(id: &str) -> Option<&'static str> {
        match id {
            "light" | "contrast-light" => Some("nord-light"),
            "dark" | "forest" | "amber" | "violet" | "contrast-dark" => Some("nord-dark"),
            _ => None,
        }
    }
    // 仅收敛已移除的项目预设；Ocean 的新配色同名同 ID，未知引用仍由校验拒绝。
    if let Some(default) = settings.get_mut("defaultSchemeId") {
        if let Some(next) = default.as_str().and_then(replacement) {
            *default = Value::String(next.into());
        }
    }
    if let Some(hosts) = settings
        .get_mut("hostSchemes")
        .and_then(Value::as_object_mut)
    {
        for scheme in hosts.values_mut() {
            if let Some(next) = scheme.as_str().and_then(replacement) {
                *scheme = Value::String(next.into());
            }
        }
    }
}

fn command(id: &str) -> bool {
    [
        "workbench.newConnection",
        "workbench.newTab",
        "workbench.closeTab",
        "workbench.nextTab",
        "workbench.previousTab",
        "settings.open",
        "workbench.newLocalTerminal",
        "workbench.toggleSidebar",
        "terminal.copy",
        "terminal.paste",
        "terminal.find",
        "terminal.scrollToBottom",
        "terminal.openLink",
    ]
    .contains(&id)
}

fn fields(value: &Value, allowed: &[&str]) -> Result<()> {
    if value
        .as_object()
        .ok_or_else(invalid)?
        .keys()
        .any(|k| !allowed.contains(&k.as_str()))
    {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn baseline() -> Value {
        json!({
            "schemaVersion": 1, "appearance": "dark", "fontFamily": DEFAULT_TERMINAL_FONT,
            "fontSize": 14, "scrollback": 5000, "defaultSchemeId": "pro",
            "confirmClose": true, "multilinePaste": true,
            "customSchemes": [], "hostSchemes": {},
            "keymap": {"schemaVersion": 2, "bindings": []}
        })
    }

    fn key_binding(command_id: &str, context: &str) -> Value {
        json!({
            "id": command_id, "commandId": command_id,
            "scope": if command_id.starts_with("terminal.") { "terminal" } else { "application" },
            "context": context,
            "trigger": {"type": "key", "key": "F8", "modifiers": ["Alt"]},
            "platform": "all", "enabled": false
        })
    }

    #[test]
    fn terminal_mouse_actions_accept_legacy_and_only_builtin_actions() {
        assert!(validate(&baseline()).is_ok());
        for key in ["terminalRightClickAction", "terminalMiddleClickAction"] {
            for action in [json!("none"), json!("paste")] {
                let mut settings = baseline();
                settings[key] = action;
                assert!(validate(&settings).is_ok());
            }
            for action in [
                json!(null),
                json!(true),
                json!("copy"),
                json!("echo unsafe"),
            ] {
                let mut settings = baseline();
                settings[key] = action;
                assert!(validate(&settings).is_err());
            }
        }
    }

    #[test]
    fn keymap_requires_current_version_commands_and_contexts() {
        let mut settings = baseline();
        settings["keymap"]["bindings"] = json!([
            key_binding("workbench.closeTab", "workbench"),
            key_binding("settings.open", "workbench"),
            key_binding("terminal.copy", "terminal")
        ]);
        assert!(validate(&settings).is_ok());
        for version in [0, 1, 3] {
            settings["keymap"]["schemaVersion"] = json!(version);
            assert!(validate(&settings).is_err());
        }
        settings["keymap"]["schemaVersion"] = json!(2);
        for binding in [
            key_binding("workspace.closeTab", "workbench"),
            key_binding("workbench.closeTab", "workspace"),
            key_binding("workbench.closeTab", "terminal"),
            key_binding("terminal.copy", "workbench"),
        ] {
            settings["keymap"]["bindings"] = json!([binding]);
            assert!(validate(&settings).is_err());
        }
    }

    #[test]
    fn system_detection_accepts_legacy_boolean_and_rejects_scripts() {
        let mut settings = baseline();
        assert!(validate(&settings).is_ok());
        for enabled in [true, false] {
            settings["detectSshSystem"] = json!(enabled);
            assert!(validate(&settings).is_ok());
        }
        settings["detectSshSystem"] = json!("true");
        assert!(validate(&settings).is_err());
        settings["detectSshSystem"] = json!(true);
        settings["systemDetectionScript"] = json!("echo arbitrary");
        assert!(validate(&settings).is_err());
    }

    #[test]
    fn hosts_view_accepts_legacy_settings_and_valid_group_preferences() {
        let mut settings = baseline();
        assert!(validate(&settings).is_ok());
        settings["hostsView"] = json!({
            "includeChildren": true,
            "expandedGroupIds": ["00000000-0000-0000-0000-000000000001"]
        });
        assert!(validate(&settings).is_ok());
        for mode in ["card", "list"] {
            for show in [true, false] {
                settings["hostsView"] = json!({
                    "includeChildren": false,
                    "expandedGroupIds": ["ungrouped"],
                    "showHosts": show,
                    "displayMode": mode
                });
                assert!(validate(&settings).is_ok());
            }
        }
    }

    #[test]
    fn hosts_view_rejects_invalid_types_duplicate_ids_and_unknown_fields() {
        let mut settings = baseline();
        for view in [
            json!({"includeChildren": "true", "expandedGroupIds": []}),
            json!({"includeChildren": false, "expandedGroupIds": [], "showHosts": "true"}),
            json!({"includeChildren": false, "expandedGroupIds": [], "showHosts": null}),
            json!({"includeChildren": false, "expandedGroupIds": [], "displayMode": "table"}),
            json!({"includeChildren": false, "expandedGroupIds": [], "displayMode": null}),
            json!({"includeChildren": false, "expandedGroupIds": ["all"]}),
            json!({"includeChildren": false, "expandedGroupIds": ["ungrouped", "ungrouped"]}),
            json!({"includeChildren": false, "expandedGroupIds": ["not-a-group-id"]}),
            json!({"includeChildren": false, "expandedGroupIds": null}),
            json!({"includeChildren": false, "expandedGroupIds": [], "extra": true}),
            json!({"includeChildren": false, "expandedGroupIds": [
                "00000000-0000-0000-0000-000000000001",
                "00000000-0000-0000-0000-000000000001"
            ]}),
        ] {
            settings["hostsView"] = view;
            assert!(validate(&settings).is_err());
        }
    }
    #[test]
    fn host_search_requires_name_and_known_unique_fields() {
        let mut settings = baseline();
        for search in [
            json!({"pinyin": true, "fields": ["name"]}),
            json!({"pinyin": false, "fields": ["name", "address", "username", "tags", "group"]}),
        ] {
            settings["hostSearch"] = search;
            assert!(validate(&settings).is_ok());
        }
        for search in [
            json!({"pinyin": true, "fields": []}),
            json!({"pinyin": true, "fields": ["address"]}),
            json!({"pinyin": true, "fields": ["name", "name"]}),
            json!({"pinyin": true, "fields": ["name", "password"]}),
            json!({"pinyin": "true", "fields": ["name"]}),
            json!({"pinyin": true, "fields": ["name"], "extra": false}),
        ] {
            settings["hostSearch"] = search;
            assert!(validate(&settings).is_err());
        }
    }
}

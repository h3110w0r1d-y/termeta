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
        "fontFamily",
        "fontSize",
        "scrollback",
        "defaultSchemeId",
        "confirmClose",
        "multilinePaste",
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
        || !settings["fontSize"]
            .as_f64()
            .is_some_and(|n| (10.0..=32.0).contains(&n))
        || !settings["scrollback"]
            .as_u64()
            .is_some_and(|n| (1000..=50000).contains(&n))
        || !settings["fontFamily"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
        || !settings["confirmClose"].is_boolean()
        || !settings["multilinePaste"].is_boolean()
    {
        return Err(invalid());
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
    if keymap["schemaVersion"] != 1 || bindings.len() > 128 {
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
            || !matches!(binding["context"].as_str(), Some("workspace" | "terminal"))
            || !command(binding["commandId"].as_str().ok_or_else(invalid)?)
        {
            return Err(invalid());
        }
        let terminal = binding["commandId"]
            .as_str()
            .is_some_and(|id| id.starts_with("terminal."));
        if binding["scope"] != if terminal { "terminal" } else { "application" }
            || binding["context"] != if terminal { "terminal" } else { "workspace" }
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
fn builtin(id: &str) -> bool {
    // 与前端内置方案名单共同维护，未知引用不能静默回退。
    [
        "dark",
        "light",
        "ocean",
        "forest",
        "amber",
        "violet",
        "contrast-dark",
        "contrast-light",
    ]
    .contains(&id)
}
fn command(id: &str) -> bool {
    [
        "workspace.newConnection",
        "workspace.closeTab",
        "workspace.nextTab",
        "workspace.previousTab",
        "settings.open",
        "workspace.newLocalTerminal",
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

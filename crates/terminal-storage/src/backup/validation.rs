//! 验证快照类型、资源限额及跨表业务关联，用于第三方备份恢复。
use super::{Cell, Data, MAX_ROWS, TABLES, invalid};
use crate::{Result, canonical_host, validate_name};
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use zeroize::Zeroizing;

fn id(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        Err(invalid())
    } else {
        Ok(())
    }
}
fn reference(value: &Cell, ids: &HashSet<String>) -> Result<()> {
    if let Some(value) = value.optional_text()? {
        if !ids.contains(value) {
            return Err(invalid());
        }
    }
    Ok(())
}
fn unique(rows: &[Vec<Cell>]) -> Result<HashSet<String>> {
    let mut ids = HashSet::new();
    for row in rows {
        let value = row[0].text()?;
        id(value)?;
        if !ids.insert(value.to_owned()) {
            return Err(invalid());
        }
    }
    Ok(ids)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyPayloadCheck {
    #[serde(deserialize_with = "secret_text")]
    private_key: Zeroizing<String>,
    #[serde(default, deserialize_with = "optional_secret_text")]
    passphrase: Option<Zeroizing<String>>,
}
fn secret_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Zeroizing<String>, D::Error> {
    String::deserialize(deserializer).map(Zeroizing::new)
}
fn optional_secret_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Zeroizing<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(|value| value.map(Zeroizing::new))
}

pub(crate) fn validate(data: &Data) -> Result<()> {
    if data.len() != TABLES.len() || data.iter().map(Vec::len).sum::<usize>() > MAX_ROWS {
        return Err(invalid());
    }
    for (table, rows) in TABLES.iter().zip(data) {
        for row in rows {
            if row.len() != table.columns().len() {
                return Err(invalid());
            }
            for (value, definition) in row.iter().zip(table.definition.split(',')) {
                match value {
                    Cell::Null if !definition.contains("NOT NULL") => (),
                    Cell::Integer(_) if definition.contains(" INTEGER") => (),
                    Cell::Blob(bytes)
                        if definition.contains(" BLOB") && bytes.len() <= 64 * 1024 =>
                    {
                        ()
                    }
                    Cell::Text(text)
                        if definition.contains(" TEXT") && text.len() <= 1024 * 1024 =>
                    {
                        ()
                    }
                    _ => return Err(invalid()),
                }
            }
        }
    }
    let group_ids = unique(&data[0])?;
    let profile_ids = unique(&data[1])?;
    let credential_ids = unique(&data[2])?;
    unique(&data[3])?;
    unique(&data[4])?;
    let tag_ids = unique(&data[5])?;
    unique(&data[7])?;
    unique(&data[8])?;
    unique(&data[9])?;
    for row in &data[9] {
        crate::snippets::validate_snippet(row[1].text()?, row[2].text()?)?;
        if row[3].integer()? < 1 {
            return Err(invalid());
        }
    }
    let group_parents: HashMap<&str, Option<&str>> = data[0]
        .iter()
        .map(|row| Ok((row[0].text()?, row[1].optional_text()?)))
        .collect::<Result<_>>()?;
    for row in &data[0] {
        validate_name(row[2].text()?)?;
        reference(&row[1], &group_ids)?;
        let mut seen = HashSet::new();
        let mut parent = Some(row[0].text()?);
        while let Some(id) = parent {
            if !seen.insert(id) || seen.len() > 128 {
                return Err(invalid());
            }
            parent = *group_parents.get(id).ok_or_else(invalid)?;
        }
        if row[4].integer()? < 1 {
            return Err(invalid());
        }
    }
    let mut names = HashSet::new();
    let profile_kinds: HashMap<&str, &str> = data[1]
        .iter()
        .map(|row| Ok((row[0].text()?, row[1].text()?)))
        .collect::<Result<_>>()?;
    let credentials: HashMap<&str, &Vec<Cell>> = data[2]
        .iter()
        .map(|row| Ok((row[0].text()?, row)))
        .collect::<Result<_>>()?;
    let ssh: HashMap<&str, &Vec<Cell>> = data[3]
        .iter()
        .map(|row| Ok((row[0].text()?, row)))
        .collect::<Result<_>>()?;
    let mut auth: HashMap<&str, Vec<terminal_core::AuthStep>> = HashMap::new();
    let mut owned_references = HashSet::new();
    for row in &data[2] {
        let kind = row[1].text()?;
        if !matches!(kind, "password" | "privateKey")
            || row[5].text()? != "ready"
            || validate_name(row[2].text()?).is_err()
            || row[4].text()?.is_empty()
        {
            return Err(invalid());
        }
        reference(&row[3], &profile_ids)?;
        if let Some(owner) = row[3].optional_text()? {
            if kind != "password" || profile_kinds.get(owner) != Some(&"ssh") {
                return Err(invalid());
            }
        }
        if kind == "password" && row[4].text()?.len() > 65536 {
            return Err(invalid());
        }
        if kind == "privateKey" {
            let key: KeyPayloadCheck = serde_json::from_str(row[4].text()?)?;
            if key.private_key.is_empty()
                || key
                    .passphrase
                    .as_ref()
                    .is_some_and(|passphrase| passphrase.len() > 65536)
            {
                return Err(invalid());
            }
        }
    }
    for row in &data[4] {
        let profile = row[1].text()?;
        if profile_kinds.get(profile) != Some(&"ssh") || row[2].integer()? != 0 {
            return Err(invalid());
        }
        let method = row[3].text()?;
        if let Some(credential) = row[4].optional_text()? {
            let credential = credentials.get(credential).ok_or_else(invalid)?;
            if credential[1].text()? != method {
                return Err(invalid());
            }
            if let Some(owner) = credential[3].optional_text()? {
                if owner != profile || !owned_references.insert(credential[0].text()?) {
                    return Err(invalid());
                }
            }
        }
        let step = match method {
            "password" => terminal_core::AuthStep::Password {
                credential_id: row[4].optional_text()?.map(str::to_owned),
            },
            "privateKey" => terminal_core::AuthStep::PrivateKey {
                credential_id: row[4].text()?.to_owned(),
            },
            _ => return Err(invalid()),
        };
        auth.entry(profile).or_default().push(step);
    }
    for row in &data[2] {
        if row[3].optional_text()?.is_some() && !owned_references.contains(row[0].text()?) {
            return Err(invalid());
        }
    }
    for row in &data[3] {
        if profile_kinds.get(row[0].text()?) != Some(&"ssh") {
            return Err(invalid());
        }
    }
    for row in &data[1] {
        let profile_id = row[0].text()?;
        let name = row[2].text()?;
        validate_name(name)?;
        if name != name.trim()
            || !names.insert(name)
            || row[5].integer()? != 1
            || row[7].integer()? < 1
            || !matches!(row[4].integer()?, 0 | 1)
        {
            return Err(invalid());
        }
        reference(&row[3], &group_ids)?;
        let config: serde_json::Value = serde_json::from_str(row[6].text()?)?;
        let object = config.as_object().ok_or_else(invalid)?;
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "startup"
                    | "icon"
                    | "systemDetectionAttempted"
                    | "systemDetectionToken"
                    | "serialConfig"
                    | "telnetConfig"
                    | "workspace"
            )
        }) {
            return Err(invalid());
        }
        if config.get("icon").is_some_and(|icon| {
            icon.as_str()
                .is_none_or(|icon| !terminal_core::HOST_SYSTEM_ICONS.contains(&icon))
        }) {
            return Err(invalid());
        }
        match row[1].text()? {
            "ssh" => {
                let target = ssh.get(profile_id).ok_or_else(invalid)?;
                let port = u16::try_from(target[3].integer()?).map_err(|_| invalid())?;
                let startup: terminal_core::SshStartup = config
                    .get("startup")
                    .map(|v| serde_json::from_value(v.clone()))
                    .transpose()?
                    .unwrap_or_default();
                let config = terminal_core::SshConfig {
                    config_version: 1,
                    system_profile_id: None,
                    address: target[1].text()?.to_owned(),
                    port,
                    username: target[4].text()?.to_owned(),
                    startup,
                    auth_plan: auth.remove(profile_id).unwrap_or_default(),
                };
                config.validate().map_err(|_| invalid())?;
                if target[2].text()? != canonical_host(&config.address) {
                    return Err(invalid());
                }
            }
            "telnet" => {
                let target: terminal_core::TelnetConfig =
                    serde_json::from_value(config["telnetConfig"].clone())?;
                target.validate().map_err(|_| invalid())?;
            }
            "serial" => {
                let target: terminal_core::SerialConfig =
                    serde_json::from_value(config["serialConfig"].clone())?;
                target.validate().map_err(|_| invalid())?;
            }
            "workspace" => {
                let workspace: crate::SavedWorkspace =
                    serde_json::from_value(config["workspace"].clone())?;
                let mut references = Vec::new();
                crate::workspaces::validate_layout(&workspace.root, 0, &mut references)?;
                if references.is_empty()
                    || workspace.focused_index >= references.len()
                    || references.iter().any(|id| {
                        profile_kinds
                            .get(id.as_str())
                            .is_some_and(|kind| !matches!(*kind, "ssh" | "telnet" | "serial"))
                    })
                {
                    return Err(invalid());
                }
            }
            _ => return Err(invalid()),
        }
    }
    if !auth.is_empty() {
        return Err(invalid());
    }
    let mut tag_names = HashSet::new();
    for row in &data[5] {
        let name = row[1].text()?;
        if name.trim().is_empty()
            || name.chars().count() > 64
            || name.chars().any(char::is_control)
            || !tag_names.insert(name)
        {
            return Err(invalid());
        }
    }
    let mut tag_links = HashSet::new();
    let mut tag_counts = HashMap::new();
    for row in &data[6] {
        reference(&row[0], &profile_ids)?;
        reference(&row[1], &tag_ids)?;
        if !tag_links.insert((row[0].text()?, row[1].text()?)) {
            return Err(invalid());
        }
        let count = tag_counts.entry(row[0].text()?).or_insert(0);
        *count += 1;
        if *count > 32 {
            return Err(invalid());
        }
    }
    for row in &data[7] {
        validate_name(row[1].text()?)?;
        if let Some(profile) = row[2].optional_text()? {
            if profile_kinds.get(profile) != Some(&"ssh") {
                return Err(invalid());
            }
        }
        let rule: terminal_core::ForwardRule = serde_json::from_str(row[3].text()?)?;
        rule.validate().map_err(|_| invalid())?;
        if row[4].integer()? < 1 {
            return Err(invalid());
        }
    }
    let mut endpoints = HashMap::new();
    for row in &data[8] {
        let host = row[1].text()?;
        let port = row[2].integer()?;
        if host.is_empty()
            || host.len() > 253
            || host.chars().any(|c| c.is_control() || c.is_whitespace())
            || host != canonical_host(host)
            || !(1..=65535).contains(&port)
            || row[3].text()?.is_empty()
            || row[3].text()?.len() > 128
            || row[4].blob()?.is_empty()
            || row[5].text()?.is_empty()
            || row[5].text()?.len() > 256
        {
            return Err(invalid());
        }
        let public_key = row[4].blob()?;
        if public_key.len() < 8 {
            return Err(invalid());
        }
        let algorithm_len =
            u32::from_be_bytes(public_key[..4].try_into().map_err(|_| invalid())?) as usize;
        if algorithm_len > 128
            || public_key.len() <= 4 + algorithm_len
            || &public_key[4..4 + algorithm_len] != row[3].text()?.as_bytes()
            || row[5].text()?
                != format!(
                    "SHA256:{}",
                    STANDARD_NO_PAD.encode(Sha256::digest(public_key))
                )
        {
            return Err(invalid());
        }
        let key = (host, port);
        if let Some(previous) = endpoints.insert(key, (row[3].text()?, row[4].blob()?)) {
            if previous != (row[3].text()?, row[4].blob()?) {
                return Err(invalid());
            }
        }
    }
    let _ = credential_ids;
    Ok(())
}

//! 白名单 SQLite 快照与非覆盖导入；备份格式与业务 schema 独立版本化。
use crate::{Result, SqliteStore, StorageError, canonical_host, now, validate_name};
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Row, ValueRef};
use std::collections::{HashMap, HashSet};
use zeroize::Zeroizing;

pub const MAX_DATABASE_BYTES: usize = 128 * 1024 * 1024;
pub(super) const MAX_ROWS: usize = 100_000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupCounts {
    pub hosts: u64,
    pub groups: u64,
    pub credentials: u64,
    pub forwards: u64,
    pub known_hosts: u64,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackupImportMode {
    Merge,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupImportSummary {
    pub counts: BackupCounts,
    pub warnings: Vec<String>,
}
/// 仅供首次云端恢复将本机备份配置与业务导入一起提交。
pub struct BackupSettingWrite {
    pub key: String,
    pub version: i64,
    pub value: serde_json::Value,
}

#[derive(Clone)]
pub(super) enum Cell {
    Null,
    Integer(i64),
    Text(Zeroizing<String>),
    Blob(Zeroizing<Vec<u8>>),
}
impl Cell {
    pub(super) fn text(&self) -> Result<&str> {
        if let Self::Text(value) = self {
            Ok(value)
        } else {
            Err(invalid())
        }
    }
    fn integer(&self) -> Result<i64> {
        if let Self::Integer(value) = self {
            Ok(*value)
        } else {
            Err(invalid())
        }
    }
    fn optional_text(&self) -> Result<Option<&str>> {
        if matches!(self, Self::Null) {
            Ok(None)
        } else {
            self.text().map(Some)
        }
    }
    fn blob(&self) -> Result<&[u8]> {
        if let Self::Blob(value) = self {
            Ok(value)
        } else {
            Err(invalid())
        }
    }
    fn set_text(&mut self, value: String) {
        *self = Self::Text(Zeroizing::new(value));
    }
    pub(super) fn size(&self) -> usize {
        match self {
            Self::Text(v) => v.len(),
            Self::Blob(v) => v.len(),
            _ => 8,
        }
    }
}
pub(super) struct Table {
    pub name: &'static str,
    definition: &'static str,
}
impl Table {
    pub(super) fn columns(&self) -> Vec<&str> {
        self.definition
            .split(',')
            .map(|column| column.split_whitespace().next().unwrap())
            .collect()
    }
    pub(super) fn schema(&self) -> String {
        format!("CREATE TABLE {} ({})", self.name, self.definition)
    }
    pub(super) fn select(&self) -> String {
        format!("SELECT {} FROM {}", self.columns().join(","), self.name)
    }
    pub(super) fn insert(&self) -> String {
        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.name,
            self.columns().join(","),
            vec!["?"; self.columns().len()].join(",")
        )
    }
}
pub(super) static TABLES: &[Table] = &[
    Table {
        name: "host_groups",
        definition: "id TEXT NOT NULL,parent_id TEXT,name TEXT NOT NULL,sort_order INTEGER NOT NULL,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL",
    },
    Table {
        name: "connection_profiles",
        definition: "id TEXT NOT NULL,kind TEXT NOT NULL,name TEXT NOT NULL,group_id TEXT,favorite INTEGER NOT NULL,config_version INTEGER NOT NULL,config_json TEXT NOT NULL,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL",
    },
    Table {
        name: "credentials",
        definition: "id TEXT NOT NULL,kind TEXT NOT NULL,label TEXT NOT NULL,owner_profile_id TEXT,secret TEXT NOT NULL,status TEXT NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL",
    },
    Table {
        name: "ssh_profiles",
        definition: "profile_id TEXT NOT NULL,address TEXT NOT NULL,canonical_host TEXT NOT NULL,port INTEGER NOT NULL,username TEXT NOT NULL",
    },
    Table {
        name: "profile_auth_steps",
        definition: "id TEXT NOT NULL,profile_id TEXT NOT NULL,sort_order INTEGER NOT NULL,method TEXT NOT NULL,credential_id TEXT",
    },
    Table {
        name: "tags",
        definition: "id TEXT NOT NULL,name TEXT NOT NULL",
    },
    Table {
        name: "profile_tags",
        definition: "profile_id TEXT NOT NULL,tag_id TEXT NOT NULL",
    },
    Table {
        name: "port_forwards",
        definition: "id TEXT NOT NULL,name TEXT NOT NULL,profile_id TEXT,rule_json TEXT NOT NULL,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL",
    },
    Table {
        name: "known_hosts",
        definition: "id TEXT NOT NULL,canonical_host TEXT NOT NULL,port INTEGER NOT NULL,key_algorithm TEXT NOT NULL,public_key BLOB NOT NULL,fingerprint_sha256 TEXT NOT NULL,trusted_at INTEGER NOT NULL,last_seen_at INTEGER",
    },
];
type Data = Vec<Vec<Vec<Cell>>>;
fn invalid() -> StorageError {
    StorageError::Validation("备份业务数据、类型或关联无效")
}
fn counts(data: &Data) -> BackupCounts {
    BackupCounts {
        hosts: data[1].len() as u64,
        groups: data[0].len() as u64,
        credentials: data[2].len() as u64,
        forwards: data[7].len() as u64,
        known_hosts: data[8].len() as u64,
    }
}
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

/// 不执行备份携带的 SQL，仅接受与固定快照 schema 完全一致的普通表。
pub fn inspect_backup_database(database: Vec<u8>) -> Result<BackupCounts> {
    let data = crate::backup_sqlite::decode(Zeroizing::new(database))?;
    validate(&data)?;
    Ok(counts(&data))
}
impl SqliteStore {
    pub async fn export_backup_database(&self) -> Result<Zeroizing<Vec<u8>>> {
        let pool = self.pool().await?;
        let mut tx = pool.begin().await?;
        let mut data = Vec::new();
        let mut total = 0;
        let mut total_bytes = 0;
        for table in TABLES {
            let filter = match table.name {
                "connection_profiles" => " WHERE kind IN ('ssh','telnet','serial','workspace')",
                "credentials" => " WHERE status='ready'",
                _ => "",
            };
            let query = format!("{}{} LIMIT {}", table.select(), filter, MAX_ROWS + 1);
            let mut stream = sqlx::query(&query).fetch(&mut *tx);
            let mut rows = Vec::new();
            use futures_util::TryStreamExt;
            while let Some(source) = stream.try_next().await? {
                total += 1;
                if total > MAX_ROWS {
                    return Err(StorageError::Validation("备份记录超过十万条限制"));
                }
                let mut row = Vec::new();
                for (index, definition) in table.definition.split(',').enumerate() {
                    let value = if source.try_get_raw(index)?.is_null() {
                        Cell::Null
                    } else if definition.contains(" INTEGER") {
                        Cell::Integer(source.try_get(index)?)
                    } else if definition.contains(" BLOB") {
                        Cell::Blob(Zeroizing::new(source.try_get(index)?))
                    } else {
                        Cell::Text(Zeroizing::new(source.try_get(index)?))
                    };
                    total_bytes += value.size();
                    if total_bytes > MAX_DATABASE_BYTES {
                        return Err(StorageError::Validation("备份数据超过 128 MiB 限制"));
                    }
                    row.push(value);
                }
                // 不把进行中的设备识别令牌复制到其他设备。
                if table.name == "connection_profiles" {
                    strip_detection_token(&mut row)?;
                }
                rows.push(row);
            }
            data.push(rows);
        }
        tx.commit().await?;
        drop(pool);
        tokio::task::spawn_blocking(move || {
            validate(&data)?;
            crate::backup_sqlite::encode(&data)
        })
        .await
        .map_err(|_| invalid())?
    }
    pub async fn import_backup_database(
        &self,
        database: Vec<u8>,
        mode: BackupImportMode,
    ) -> Result<BackupImportSummary> {
        self.import_backup_database_with_setting(database, mode, None)
            .await
    }
    pub async fn import_backup_database_with_setting(
        &self,
        database: Vec<u8>,
        _mode: BackupImportMode,
        setting: Option<BackupSettingWrite>,
    ) -> Result<BackupImportSummary> {
        let database = Zeroizing::new(database);
        let mut data = tokio::task::spawn_blocking(move || {
            let data = crate::backup_sqlite::decode(database)?;
            validate(&data)?;
            Ok::<_, StorageError>(data)
        })
        .await
        .map_err(|_| invalid())??;
        let setting = setting
            .map(|setting| {
                if setting.key != "backup" || setting.version < 1 {
                    return Err(invalid());
                }
                let encoded = Zeroizing::new(serde_json::to_string(&setting.value)?);
                if encoded.len() > 1024 * 1024 {
                    return Err(invalid());
                }
                Ok((setting.version, encoded))
            })
            .transpose()?;
        let pool = self.pool().await?;
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut summary = BackupImportSummary {
            counts: counts(&data),
            warnings: Vec::new(),
        };
        remap(&mut data, &mut tx).await?;
        // 父组必须先于子组插入；验证阶段已排除环，排序不依赖来源行顺序。
        let mut groups = std::mem::take(&mut data[0]);
        let parents: HashMap<String, Option<String>> = groups
            .iter()
            .map(|row| {
                Ok((
                    row[0].text()?.to_owned(),
                    row[1].optional_text()?.map(str::to_owned),
                ))
            })
            .collect::<Result<_>>()?;
        groups.sort_by_key(|row| {
            let mut depth = 0;
            let mut parent = row[1].optional_text().ok().flatten();
            while let Some(id) = parent {
                depth += 1;
                parent = parents.get(id).and_then(|parent| parent.as_deref());
            }
            depth
        });
        for row in groups {
            insert(&mut tx, &TABLES[0], &row).await?;
        }
        for index in 1..TABLES.len() {
            for row in &data[index] {
                if index == 8 {
                    // 当前端点存在其他密钥时跳过，不能用导入增加第二个密钥绕过变更检测。
                    let existing = sqlx::query("SELECT key_algorithm,public_key FROM known_hosts WHERE canonical_host=? AND port=?")
                        .bind(row[1].text()?).bind(row[2].integer()?).fetch_all(&mut *tx).await?;
                    if !existing.is_empty() {
                        let same = existing.iter().any(|r| {
                            r.get::<String, _>(0) == row[3].text().unwrap_or_default()
                                && r.get::<Vec<u8>, _>(1).as_slice()
                                    == row[4].blob().unwrap_or_default()
                        });
                        summary.counts.known_hosts -= 1;
                        if !same {
                            summary.warnings.push(format!(
                                "已跳过主机信任冲突：{}:{}",
                                row[1].text()?,
                                row[2].integer()?
                            ));
                        }
                        continue;
                    }
                }
                if index == 5 {
                    let exists: bool =
                        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tags WHERE id=?)")
                            .bind(row[0].text()?)
                            .fetch_one(&mut *tx)
                            .await?;
                    if exists {
                        continue;
                    }
                }
                insert(&mut tx, &TABLES[index], row).await?;
            }
        }
        if let Some((version, encoded)) = setting {
            sqlx::query("INSERT INTO settings(key,value_json,value_version,updated_at) VALUES('backup',?,?,?) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,value_version=excluded.value_version,updated_at=excluded.updated_at")
                .bind(encoded.as_str()).bind(version).bind(now()).execute(&mut *tx).await?;
        }
        if !sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&mut *tx)
            .await?
            .is_empty()
        {
            return Err(invalid());
        }
        tx.commit().await?;
        Ok(summary)
    }
}
async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    table: &Table,
    row: &[Cell],
) -> Result<()> {
    let sql = table.insert();
    let mut query = sqlx::query(&sql);
    for value in row {
        query = match value {
            Cell::Null => query.bind(Option::<String>::None),
            Cell::Integer(v) => query.bind(*v),
            Cell::Text(v) => query.bind(v.as_str()),
            Cell::Blob(v) => query.bind(v.as_slice()),
        };
    }
    query.execute(&mut **tx).await?;
    Ok(())
}
fn strip_detection_token(row: &mut [Cell]) -> Result<()> {
    let mut config: serde_json::Value = serde_json::from_str(row[6].text()?)?;
    if let Some(object) = config.as_object_mut() {
        object.remove("systemDetectionToken");
    }
    row[6].set_text(serde_json::to_string(&config)?);
    Ok(())
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

fn validate(data: &Data) -> Result<()> {
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

fn mapped(value: &mut Cell, mapping: &HashMap<String, String>) -> Result<()> {
    if let Some(old) = value.optional_text()? {
        value.set_text(mapping.get(old).ok_or_else(invalid)?.clone());
    }
    Ok(())
}
fn remap_layout(node: &mut crate::SavedLayout, profiles: &mut HashMap<String, String>) {
    match node {
        crate::SavedLayout::Connection { profile_id } => {
            // 已删除的来源引用保持悬空，禁止与本机恰巧相同的旧 ID 绑定。
            *profile_id = profiles
                .entry(profile_id.clone())
                .or_insert_with(|| uuid::Uuid::new_v4().to_string())
                .clone();
        }
        crate::SavedLayout::Split { first, second, .. } => {
            remap_layout(first, profiles);
            remap_layout(second, profiles);
        }
    }
}
async fn remap(data: &mut Data, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<()> {
    let mapping = |rows: &Vec<Vec<Cell>>| -> Result<HashMap<String, String>> {
        rows.iter()
            .map(|row| Ok((row[0].text()?.to_owned(), uuid::Uuid::new_v4().to_string())))
            .collect()
    };
    let groups = mapping(&data[0])?;
    let mut profiles = mapping(&data[1])?;
    let credentials = mapping(&data[2])?;
    let mut tags = mapping(&data[5])?;
    for row in &data[5] {
        if let Some(id) = sqlx::query_scalar::<_, String>("SELECT id FROM tags WHERE name=?")
            .bind(row[1].text()?)
            .fetch_optional(&mut **tx)
            .await?
        {
            tags.insert(row[0].text()?.to_owned(), id);
        }
    }
    let names:Vec<String>=sqlx::query_scalar("SELECT trim(name) FROM connection_profiles WHERE kind IN ('ssh','telnet','serial','workspace')").fetch_all(&mut **tx).await?;
    let mut names: HashSet<String> = names.into_iter().collect();
    for row in &mut data[0] {
        mapped(&mut row[0], &groups)?;
        mapped(&mut row[1], &groups)?;
        row[4] = Cell::Integer(1);
    }
    for row in &mut data[1] {
        mapped(&mut row[0], &profiles)?;
        mapped(&mut row[3], &groups)?;
        let base = row[2].text()?.to_owned();
        if !names.insert(base.clone()) {
            let mut number = 1;
            loop {
                let suffix = format!(" (同步 {number})");
                let candidate = format!(
                    "{}{}",
                    base.chars()
                        .take(128 - suffix.chars().count())
                        .collect::<String>(),
                    suffix
                );
                if names.insert(candidate.clone()) {
                    row[2].set_text(candidate);
                    break;
                }
                number += 1;
            }
        }
        if row[1].text()? == "workspace" {
            let mut config: serde_json::Value = serde_json::from_str(row[6].text()?)?;
            let mut workspace: crate::SavedWorkspace =
                serde_json::from_value(config["workspace"].clone())?;
            remap_layout(&mut workspace.root, &mut profiles);
            config["workspace"] = serde_json::to_value(workspace)?;
            row[6].set_text(serde_json::to_string(&config)?);
        }
        strip_detection_token(row)?;
        row[7] = Cell::Integer(1);
    }
    for row in &mut data[2] {
        mapped(&mut row[0], &credentials)?;
        mapped(&mut row[3], &profiles)?;
    }
    for row in &mut data[3] {
        mapped(&mut row[0], &profiles)?;
    }
    for row in &mut data[4] {
        row[0].set_text(uuid::Uuid::new_v4().to_string());
        mapped(&mut row[1], &profiles)?;
        mapped(&mut row[4], &credentials)?;
    }
    for row in &mut data[5] {
        mapped(&mut row[0], &tags)?;
    }
    for row in &mut data[6] {
        mapped(&mut row[0], &profiles)?;
        mapped(&mut row[1], &tags)?;
    }
    for row in &mut data[7] {
        row[0].set_text(uuid::Uuid::new_v4().to_string());
        mapped(&mut row[2], &profiles)?;
        row[4] = Cell::Integer(1);
    }
    for row in &mut data[8] {
        row[0].set_text(uuid::Uuid::new_v4().to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HostDraft, SavedLayout, SavedWorkspace, WorkspaceDraft};
    struct TestDirectory(std::path::PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("termeta-backup-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        async fn store(&self, name: &str) -> SqliteStore {
            SqliteStore::open(self.0.join(name)).await.unwrap()
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn draft(credential_id: Option<String>) -> HostDraft {
        HostDraft {
            startup: terminal_core::SshStartup::default(),
            kind: "ssh".into(),
            serial_config: None,
            icon: "ssh".into(),
            id: None,
            revision: None,
            name: "备份主机".into(),
            address: "example.com".into(),
            port: 22,
            username: "test".into(),
            group_id: None,
            favorite: false,
            tags: vec!["服务器".into()],
            auth_plan: vec![terminal_core::AuthStep::Password { credential_id }],
        }
    }
    #[tokio::test(flavor = "current_thread")]
    async fn merge_preserves_local_settings_and_remaps_every_relationship() {
        let directory = TestDirectory::new();
        let source = directory.store("source.sqlite3").await;
        let credential = source
            .begin_credential("password", "公共密码")
            .await
            .unwrap();
        source
            .set_credential_secret(&credential, "仅备份密文包含此内容")
            .await
            .unwrap();
        let host = source
            .save_host(draft(Some(credential.clone())))
            .await
            .unwrap();
        source
            .save_workspace(WorkspaceDraft {
                name: "双窗格".into(),
                expected_revision: None,
                workspace: SavedWorkspace {
                    root: SavedLayout::Connection {
                        profile_id: host.id.clone(),
                    },
                    focused_index: 0,
                },
            })
            .await
            .unwrap();
        source
            .set_setting("app", 1, &serde_json::json!({"theme":"source"}))
            .await
            .unwrap();
        source
            .set_setting("backup", 2, &serde_json::json!({"target":{"secret":"do-not-export"},"backupPassword":"backup-password-do-not-export"}))
            .await
            .unwrap();
        let snapshot = source.export_backup_database().await.unwrap();
        assert!(snapshot.starts_with(b"SQLite format 3\0"));
        assert!(
            !snapshot
                .windows(b"do-not-export".len())
                .any(|bytes| bytes == b"do-not-export")
        );
        assert!(
            !snapshot
                .windows(b"backup-password-do-not-export".len())
                .any(|bytes| bytes == b"backup-password-do-not-export")
        );
        let target = directory.store("target.sqlite3").await;
        target.save_host(draft(None)).await.unwrap();
        target
            .set_setting("app", 1, &serde_json::json!({"theme":"local"}))
            .await
            .unwrap();
        let summary = target
            .import_backup_database_with_setting(
                snapshot.to_vec(),
                BackupImportMode::Merge,
                Some(BackupSettingWrite {
                    key: "backup".into(),
                    version: 2,
                    value: serde_json::json!({"target":{"provider":"gist"},"backupPassword":"initialization-password-placeholder"}),
                }),
            )
            .await
            .unwrap();
        assert_eq!(summary.counts.hosts, 2);
        assert_eq!(
            target.get_setting("app").await.unwrap().unwrap().value["theme"],
            "local"
        );
        assert_eq!(
            target.get_setting("backup").await.unwrap().unwrap().value["target"]["provider"],
            "gist"
        );
        let cloud_config = target.get_setting("backup").await.unwrap().unwrap();
        assert_eq!(cloud_config.value_version, 2);
        assert_eq!(
            cloud_config.value["backupPassword"],
            "initialization-password-placeholder"
        );
        let imported = target
            .list_hosts("")
            .await
            .unwrap()
            .into_iter()
            .find(|host| host.name == "备份主机 (同步 1)")
            .unwrap();
        assert_ne!(imported.id, host.id);
        let connection = target.get_host_connection(&imported.id).await.unwrap();
        let terminal_core::AuthStep::Password {
            credential_id: Some(imported_credential),
        } = &connection.auth_plan[0]
        else {
            panic!("凭据关联缺失")
        };
        assert_ne!(imported_credential, &credential);
        assert_eq!(
            target
                .credential_secret(imported_credential)
                .await
                .unwrap()
                .as_str(),
            "仅备份密文包含此内容"
        );
        let workspace = target
            .list_hosts("")
            .await
            .unwrap()
            .into_iter()
            .find(|host| host.kind == "workspace")
            .unwrap()
            .workspace
            .unwrap();
        let SavedLayout::Connection { profile_id } = workspace.root else {
            panic!("工作区布局错误")
        };
        assert_eq!(profile_id, imported.id);
    }
    #[tokio::test(flavor = "current_thread")]
    async fn trust_conflicts_are_skipped_and_malformed_import_is_atomic() {
        let directory = TestDirectory::new();
        let source = directory.store("source.sqlite3").await;
        let target = directory.store("target.sqlite3").await;
        let trust = |key: u8| {
            let mut public_key = Vec::new();
            public_key.extend_from_slice(&11u32.to_be_bytes());
            public_key.extend_from_slice(b"ssh-ed25519");
            public_key.extend_from_slice(&32u32.to_be_bytes());
            public_key.extend_from_slice(&[key; 32]);
            crate::TrustedHostKey {
                id: uuid::Uuid::new_v4().to_string(),
                canonical_host: "example.com".into(),
                port: 22,
                key_algorithm: "ssh-ed25519".into(),
                fingerprint_sha256: format!(
                    "SHA256:{}",
                    STANDARD_NO_PAD.encode(Sha256::digest(&public_key))
                ),
                public_key,
                trusted_at: 1,
            }
        };
        source.trust_key(trust(1)).await.unwrap();
        target.trust_key(trust(2)).await.unwrap();
        let bytes = source.export_backup_database().await.unwrap();
        let summary = target
            .import_backup_database(bytes.to_vec(), BackupImportMode::Merge)
            .await
            .unwrap();
        assert_eq!(summary.counts.known_hosts, 0);
        assert_eq!(summary.warnings.len(), 1);
        assert_eq!(
            target.list_known_hosts().await.unwrap()[0].public_key,
            trust(2).public_key
        );
        let before = target.list_hosts("").await.unwrap().len();
        assert!(
            target
                .import_backup_database_with_setting(
                    vec![0; 100],
                    BackupImportMode::Merge,
                    Some(BackupSettingWrite {
                        key: "backup".into(),
                        version: 1,
                        value: serde_json::json!({"token":"must-not-save"})
                    })
                )
                .await
                .is_err()
        );
        assert_eq!(target.list_hosts("").await.unwrap().len(), before);
        assert!(target.get_setting("backup").await.unwrap().is_none());
    }
    #[test]
    fn snapshot_rejects_untrusted_schema_and_group_cycles() {
        let mut data: Data = TABLES.iter().map(|_| Vec::new()).collect();
        let bytes = crate::backup_sqlite::encode(&data).unwrap();
        assert_eq!(
            inspect_backup_database(bytes.to_vec()).unwrap(),
            BackupCounts::default()
        );
        let mut damaged = bytes.to_vec();
        let from = b"CREATE TABLE tags";
        let start = damaged
            .windows(from.len())
            .position(|bytes| bytes == from)
            .unwrap();
        damaged[start..start + 6].copy_from_slice(b"CREATE");
        damaged[start + 13..start + 17].copy_from_slice(b"evil");
        assert!(inspect_backup_database(damaged).is_err());
        data[0].push(vec![
            Cell::Text(Zeroizing::new("group".into())),
            Cell::Text(Zeroizing::new("group".into())),
            Cell::Text(Zeroizing::new("循环分组".into())),
            Cell::Integer(0),
            Cell::Integer(1),
            Cell::Integer(1),
            Cell::Integer(1),
        ]);
        assert!(validate(&data).is_err());
    }
}

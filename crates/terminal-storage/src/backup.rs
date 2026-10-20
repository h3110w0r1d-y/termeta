//! 白名单 SQLite 快照与非覆盖导入；备份格式与业务 schema 独立版本化。
use crate::{Result, SqliteStore, StorageError, now};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use zeroize::Zeroizing;

mod database;
mod merge;
mod schema;
#[cfg(test)]
mod tests;
mod validation;

pub(super) use database::{insert, read_data, strip_detection_token};
use merge::remap;
pub(super) use schema::{Cell, Data, TABLES, Table};
pub(super) use validation::validate;

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
    #[serde(default)]
    pub snippets: u64,
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
/// 将受控的本机备份配置与业务导入同事务提交，不接受其他设置。
pub struct BackupSettingWrite {
    pub key: String,
    pub version: i64,
    pub value: serde_json::Value,
}

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
        snippets: data[9].len() as u64,
    }
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
        let data = read_data(&mut tx).await?;
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

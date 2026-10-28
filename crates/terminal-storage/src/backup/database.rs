//! 白名单业务快照的事务读写及不跨设备传播的元数据清理。
use super::{Cell, Data, MAX_DATABASE_BYTES, MAX_ROWS, TABLES, Table};
use crate::{Result, StorageError};
use sqlx::{AssertSqlSafe, Row, ValueRef};
use zeroize::Zeroizing;

pub(crate) async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    table: &Table,
    row: &[Cell],
) -> Result<()> {
    let sql = table.insert();
    // 表名和列名仅来自内部白名单，备份单元格继续通过参数绑定写入。
    let mut query = sqlx::query(AssertSqlSafe(sql));
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
pub(crate) fn strip_detection_token(row: &mut [Cell]) -> Result<()> {
    let mut config: serde_json::Value = serde_json::from_str(row[6].text()?)?;
    if let Some(object) = config.as_object_mut() {
        object.remove("systemDetectionToken");
    }
    row[6].set_text(serde_json::to_string(&config)?);
    Ok(())
}

/// 按白名单导出业务数据，调用方持有一致的数据库事务。
pub(crate) async fn read_data(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<Data> {
    let mut data = Vec::new();
    let mut total = 0;
    let mut total_bytes = 0;
    for table in TABLES {
        let filter = match table.name {
            "connection_profiles" => " WHERE kind IN ('ssh','telnet','serial','workspace')",
            "credentials" => " WHERE status='ready'",
            "ssh_profiles" | "profile_auth_steps" | "profile_tags" => {
                " WHERE profile_id IN (SELECT id FROM connection_profiles WHERE kind IN ('ssh','telnet','serial','workspace'))"
            }
            _ => "",
        };
        let query = format!("{}{} LIMIT {}", table.select(), filter, MAX_ROWS + 1);
        // 查询只组合内部表定义、固定筛选和常量限额，不接收备份内容作为 SQL。
        let mut stream = sqlx::query(AssertSqlSafe(query)).fetch(&mut **tx);
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
    Ok(data)
}

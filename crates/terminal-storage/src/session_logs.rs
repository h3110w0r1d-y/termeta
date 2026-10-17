use crate::{Result, SqliteStore, StorageError, now};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

// 只在关闭终端视图时传入 xterm 的最终文本；正文不经过列表或连接事件。
pub const MAX_SESSION_LOG_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionLogDraft {
    pub id: String,
    pub protocol: String,
    pub name: String,
    pub username: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub started_at: i64,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLogSummary {
    pub id: String,
    pub protocol: String,
    pub name: String,
    pub username: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub started_at: i64,
    pub closed_at: i64,
    pub byte_length: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLog {
    #[serde(flatten)]
    pub summary: SessionLogSummary,
    pub content: String,
}

pub(crate) async fn ensure_session_logs_schema(connection: &mut SqliteConnection) -> Result<()> {
    // 新日志与旧连接历史相互独立，不读取或迁移旧历史，也不关联可删除的主机配置。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS session_logs (
            id TEXT PRIMARY KEY NOT NULL,
            protocol TEXT NOT NULL CHECK(protocol IN ('ssh','telnet','serial','local')),
            name TEXT NOT NULL,
            username TEXT,
            host TEXT,
            port INTEGER CHECK(port IS NULL OR port BETWEEN 1 AND 65535),
            started_at INTEGER NOT NULL,
            closed_at INTEGER NOT NULL,
            byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
            content TEXT NOT NULL
        )",
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS session_logs_closed_at ON session_logs(closed_at DESC, id)",
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

fn validate_snapshot_text(value: &str, limit: usize) -> bool {
    value.chars().count() <= limit && !value.chars().any(char::is_control)
}

fn summary(row: &SqliteRow) -> Result<SessionLogSummary> {
    Ok(SessionLogSummary {
        id: row.try_get("id")?,
        protocol: row.try_get("protocol")?,
        name: row.try_get("name")?,
        username: row.try_get("username")?,
        host: row.try_get("host")?,
        port: row
            .try_get::<Option<i64>, _>("port")?
            .map(|value| value as u16),
        started_at: row.try_get("started_at")?,
        closed_at: row.try_get("closed_at")?,
        byte_length: row.try_get("byte_length")?,
    })
}

impl SqliteStore {
    pub async fn save_session_log(&self, draft: SessionLogDraft) -> Result<SessionLogSummary> {
        let closed_at = now();
        if uuid::Uuid::parse_str(&draft.id).is_err()
            || !matches!(
                draft.protocol.as_str(),
                "ssh" | "telnet" | "serial" | "local"
            )
            || draft.name.trim().is_empty()
            || !validate_snapshot_text(&draft.name, 1024)
            || draft
                .username
                .as_deref()
                .is_some_and(|value| !validate_snapshot_text(value, 256))
            || draft
                .host
                .as_deref()
                .is_some_and(|value| !validate_snapshot_text(value, 4096))
            || draft.port == Some(0)
            || draft.started_at < 0
        {
            return Err(StorageError::Validation("会话日志连接信息无效"));
        }
        if draft.content.len() > MAX_SESSION_LOG_BYTES {
            return Err(StorageError::Validation(
                "会话日志超过 128 MiB，尚未保存，请保留终端并重试",
            ));
        }
        let mut transaction = self.pool().await?.begin_with("BEGIN IMMEDIATE").await?;
        // 提交成功但应答丢失后终端可能继续使用；重试更新同一行，保留最新完整快照。
        sqlx::query("INSERT INTO session_logs(id,protocol,name,username,host,port,started_at,closed_at,byte_length,content) VALUES(?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET protocol=excluded.protocol,name=excluded.name,username=excluded.username,host=excluded.host,port=excluded.port,started_at=excluded.started_at,closed_at=excluded.closed_at,byte_length=excluded.byte_length,content=excluded.content")
            .bind(&draft.id)
            .bind(&draft.protocol)
            .bind(&draft.name)
            .bind(&draft.username)
            .bind(&draft.host)
            .bind(draft.port.map(i64::from))
            .bind(draft.started_at)
            .bind(closed_at.max(draft.started_at))
            .bind(draft.content.len() as i64)
            .bind(&draft.content)
            .execute(&mut *transaction)
            .await?;
        let row = sqlx::query("SELECT id,protocol,name,username,host,port,started_at,closed_at,byte_length FROM session_logs WHERE id=?")
            .bind(&draft.id)
            .fetch_one(&mut *transaction)
            .await?;
        let saved = summary(&row)?;
        // 连接使用 synchronous=FULL；提交完成才允许调用方释放终端或继续退出。
        transaction.commit().await?;
        Ok(saved)
    }

    pub async fn list_session_logs(&self) -> Result<Vec<SessionLogSummary>> {
        sqlx::query("SELECT id,protocol,name,username,host,port,started_at,closed_at,byte_length FROM session_logs ORDER BY closed_at DESC,rowid DESC")
            .fetch_all(&*self.pool().await?)
            .await?
            .iter()
            .map(summary)
            .collect()
    }

    pub async fn read_session_log(&self, id: &str) -> Result<SessionLog> {
        let row = sqlx::query("SELECT id,protocol,name,username,host,port,started_at,closed_at,byte_length,content FROM session_logs WHERE id=?")
            .bind(id)
            .fetch_optional(&*self.pool().await?)
            .await?
            .ok_or(StorageError::NotFound)?;
        Ok(SessionLog {
            summary: summary(&row)?,
            content: row.try_get("content")?,
        })
    }

    pub async fn delete_session_log(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM session_logs WHERE id=?")
            .bind(id)
            .execute(&*self.pool().await?)
            .await?;
        Ok(())
    }

    pub async fn clear_session_logs(&self) -> Result<()> {
        sqlx::query("DELETE FROM session_logs")
            .execute(&*self.pool().await?)
            .await?;
        Ok(())
    }
}

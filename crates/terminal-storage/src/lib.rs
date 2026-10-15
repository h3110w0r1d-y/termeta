mod credentials;
mod groups;
mod hosts;
mod key_files;
mod metadata;
mod model;
#[cfg(test)]
mod tests;

pub use key_files::read_key_file;
pub use model::*;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("数据库操作失败")]
    Database(#[from] sqlx::Error),
    #[error("数据库迁移失败：{0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("数据格式无效")]
    Json(#[from] serde_json::Error),
    #[error("数据库文件或备份权限操作失败")]
    Io(#[from] std::io::Error),
    #[error("记录不存在")]
    NotFound,
    #[error("记录已被修改，请重新加载最新版本")]
    Conflict,
    #[error("{0}")]
    Validation(&'static str),
}
pub type Result<T> = std::result::Result<T, StorageError>;

pub struct SqliteStore {
    pub(crate) pool: SqlitePool,
}
pub type Repository = SqliteStore;

impl SqliteStore {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let existing = path.exists();
        let mut applied_version = 0;
        if existing {
            // 先只读核验未来版本，避免设置 WAL 时修改不受支持的数据库。
            let probe = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(SqliteConnectOptions::new().filename(path).read_only(true))
                .await?;
            let version: i64 = sqlx::query_scalar("PRAGMA user_version")
                .fetch_one(&probe)
                .await?;
            let migrations: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
            )
            .fetch_one(&probe)
            .await?;
            let latest = if migrations > 0 {
                sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM _sqlx_migrations")
                    .fetch_one(&probe)
                    .await?
                    .unwrap_or(0)
            } else {
                0
            };
            probe.close().await;
            if version > SCHEMA_VERSION || latest > SCHEMA_VERSION {
                return Err(StorageError::Validation("数据库来自更高版本，拒绝写入"));
            }
            applied_version = latest;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(3));
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await?;
        private_file(path)?;
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await?;
        if version > SCHEMA_VERSION {
            return Err(StorageError::Validation("数据库来自更高版本，拒绝写入"));
        }
        if existing && applied_version < SCHEMA_VERSION {
            // VACUUM INTO 读取一致性快照，包含 WAL，不直接复制活跃数据库文件。
            let backup = path.with_extension(format!("backup-{}.sqlite3", now()));
            sqlx::query("VACUUM INTO ?")
                .bind(backup.to_string_lossy().as_ref())
                .execute(&pool)
                .await?;
            private_file(&backup)?;
            prune_backups(path)?;
        }
        sqlx::migrate!("./migrations").run(&pool).await?;
        sqlx::query(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))
            .execute(&pool)
            .await?;
        sqlx::query("UPDATE session_history SET ended_at=?,end_reason='appInterrupted' WHERE ended_at IS NULL").bind(now()).execute(&pool).await?;
        let store = Self { pool };
        store.prune_history(90).await?;
        Ok(store)
    }
}

pub(crate) fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
pub fn canonical_host(address: &str) -> String {
    let address = address
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.');
    address
        .parse::<std::net::IpAddr>()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| address.to_ascii_lowercase())
}
fn private_file(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    let _ = path;
    Ok(())
}
fn prune_backups(path: &Path) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let prefix = format!(
        "{}.backup-",
        path.file_stem().unwrap_or_default().to_string_lossy()
    );
    let mut files = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stamp) = name
            .strip_prefix(&prefix)
            .and_then(|s| s.strip_suffix(".sqlite3"))
        else {
            continue;
        };
        if stamp.parse::<i64>().is_ok() && entry.file_type()?.is_file() {
            files.push(entry.path());
        }
    }
    files.sort();
    let excess = files.len().saturating_sub(5);
    for backup in files.into_iter().take(excess) {
        std::fs::remove_file(backup)?;
    }
    Ok(())
}
pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err(StorageError::Validation(
            "名称须为 1–128 个不含控制字符的字符",
        ));
    }
    Ok(())
}

pub mod backup;
mod backup_sqlite;
pub use backup::{
    BackupCounts, BackupImportMode, BackupImportSummary, BackupSettingWrite, MAX_DATABASE_BYTES,
    inspect_backup_database,
};
mod credentials;
mod password_change;
pub use password_change::PasswordChangeOutcome;
mod snippets;
pub use snippets::*;
mod forwarding;
pub use forwarding::*;
mod groups;
mod hosts;
mod workspaces;
use workspaces::ensure_unique_profile_name;
mod key_files;
mod metadata;
mod session_logs;
pub use session_logs::*;
mod model;
mod system;

pub use key_files::read_key_file;
pub use model::*;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{ConnectOptions, SqlitePool};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const SCHEMA_VERSION: i64 = 11;

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
    #[error("数据保险库尚未解锁")]
    Locked,
    #[error("记录不存在")]
    NotFound,
    #[error("记录已被修改，请重新加载最新版本")]
    Conflict,
    #[error("{0}")]
    Validation(&'static str),
}
pub type Result<T> = std::result::Result<T, StorageError>;

pub struct SqliteStore {
    pool: tokio::sync::RwLock<Option<SqlitePool>>,
    unlocked: AtomicBool,
    path: PathBuf,
}
impl SqliteStore {
    /// 构造阶段不打开数据库；解锁前任何仓库访问都明确返回 Locked。
    pub fn locked(path: impl AsRef<Path>) -> Self {
        Self {
            pool: tokio::sync::RwLock::new(None),
            unlocked: AtomicBool::new(false),
            path: path.as_ref().to_owned(),
        }
    }
    pub fn is_unlocked(&self) -> bool {
        self.unlocked.load(Ordering::Acquire)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) async fn pool(&self) -> Result<tokio::sync::RwLockReadGuard<'_, SqlitePool>> {
        // 生命周期门闩只协调本地数据库切换，不占用会话表或网络操作的全局锁。
        tokio::sync::RwLockReadGuard::try_map(self.pool.read().await, |slot| slot.as_ref())
            .map_err(|_| StorageError::Locked)
    }
    /// 调用方持有初始化专用锁；验证密钥与完整性后才发布可供业务访问的连接池。
    pub async fn unlock(&self, key: &[u8; 32]) -> Result<()> {
        let mut slot = self.pool.write().await;
        if slot.is_some() {
            return Ok(());
        }
        if !self.path.exists() {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options.open(&self.path)?;
        }
        let options = Self::connect_options(&self.path, key, SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await?;
        let result = Self::prepare(&pool, &self.path).await;
        if let Err(error) = result {
            pool.close().await;
            return Err(error);
        }
        *slot = Some(pool);
        self.unlocked.store(true, Ordering::Release);
        Ok(())
    }
    fn connect_options(
        path: &Path,
        key: &[u8; 32],
        journal: SqliteJournalMode,
    ) -> SqliteConnectOptions {
        let encoded = zeroize::Zeroizing::new(hex::encode(key));
        SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(false)
            // SQLx 保证 key 是第一条 PRAGMA；禁用语句日志，避免密钥出现在诊断中。
            .pragma("key", format!("\"x'{}'\"", encoded.as_str()))
            .pragma("cipher_compatibility", "4")
            .pragma("cipher_memory_security", "ON")
            .pragma("temp_store", "MEMORY")
            .pragma("secure_delete", "ON")
            .journal_mode(journal)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(3))
            .disable_statement_logging()
    }
    async fn validate(pool: &SqlitePool, path: &Path) -> Result<()> {
        let cipher: Option<String> = sqlx::query_scalar("PRAGMA cipher_version")
            .fetch_optional(pool)
            .await?;
        if cipher
            .as_deref()
            .is_none_or(|value| !value.starts_with("4."))
        {
            return Err(StorageError::Validation(
                "缺少 SQLCipher 4 加密支持，拒绝打开数据库",
            ));
        }
        // 先读取加密页，错误密钥或损坏不得进入迁移及写入流程。
        sqlx::query("SELECT count(*) FROM sqlite_master")
            .execute(pool)
            .await?;
        let integrity: Vec<String> = sqlx::query_scalar("PRAGMA cipher_integrity_check")
            .fetch_all(pool)
            .await?;
        if !integrity.is_empty() {
            return Err(StorageError::Validation("加密数据库完整性校验失败"));
        }
        let check: String = sqlx::query_scalar("PRAGMA quick_check")
            .fetch_one(pool)
            .await?;
        if check != "ok" {
            return Err(StorageError::Validation("数据库完整性校验失败"));
        }
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(pool)
            .await?;
        if ![0, 2, 3, 4, 5, 6, 7, 8, 9, 10, SCHEMA_VERSION].contains(&version) {
            return Err(StorageError::Validation(
                "数据库格式不兼容，仅支持当前加密格式及 schema 2–10 升级",
            ));
        }
        private_file(path)?;
        Ok(())
    }
    async fn prepare(pool: &SqlitePool, path: &Path) -> Result<()> {
        Self::validate(pool, path).await?;
        // 仅初始化阶段持有的同一连接关闭外键；业务连接池尚未发布。
        // SQLite 重建父表必须在迁移事务外关闭外键，否则会删除子表/置空历史引用。
        let mut connection = pool.acquire().await?;
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&mut *connection)
            .await?;
        // 已取得连接时直接迁移，避免 run 的 Acquire 生命周期约束使解锁 future 无法满足 Send。
        // 不限制目标版本且实际执行迁移 SQL，不能只把待执行项登记为已应用。
        let migrated = sqlx::migrate!("./migrations")
            .run_direct(None, &mut *connection, false)
            .await;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut *connection)
            .await?;
        migrated?;
        snippets::ensure_snippets_schema(&mut connection).await?;
        let violations = sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&mut *connection)
            .await?;
        if !violations.is_empty() {
            return Err(StorageError::Validation("数据库外键完整性校验失败"));
        }
        session_logs::ensure_session_logs_schema(&mut connection).await?;
        Ok(())
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
pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err(StorageError::Validation(
            "名称须为 1–128 个不含控制字符的字符",
        ));
    }
    Ok(())
}

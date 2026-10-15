use super::*;
use sqlx::migrate::Migrator;
use std::path::PathBuf;

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("termvexa-migration-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn upgrades_original_database_without_rewriting_migration_or_granting_old_keys() {
    let directory = TestDirectory::new();
    let path = directory.0.join("termvexa.sqlite3");
    let legacy = directory.0.join("legacy");
    std::fs::create_dir(&legacy).unwrap();
    std::fs::write(
        legacy.join("0001_initial.sql"),
        include_str!("../migrations/0001_initial.sql"),
    )
    .unwrap();
    let pool = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    Migrator::new(legacy.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    // 固定已在用户数据库应用的校验和，防止重写旧迁移再次破坏升级。
    let checksum: String =
        sqlx::query_scalar("SELECT hex(checksum) FROM _sqlx_migrations WHERE version=1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        checksum,
        "FF38F38EC75B9A99E4DDAA4586E5BE24D0BE55E6AC3BA985D1566741AAF31DB2B248E733B72581B97E9AC202BD414B5E"
    );
    sqlx::query("PRAGMA user_version = 1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO settings(key,value_json,value_version,updated_at) VALUES('retained','true',1,0)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO key_files(id,path,display_name,created_at,updated_at) VALUES('legacy','unused','旧私钥',0,0)")
        .execute(&pool).await.unwrap();
    pool.close().await;

    let store = SqliteStore::open(&path).await.unwrap();
    assert_eq!(
        store.get_setting("retained").await.unwrap().unwrap().value,
        serde_json::json!(true)
    );
    let key = store.get_key_file("legacy").await.unwrap();
    assert!(key.content_fingerprint().is_empty());
    assert!(
        matches!(read_key_file(Path::new(&key.path), Some(key.content_fingerprint())), Err(StorageError::Validation(message)) if message.contains("重新选择"))
    );
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    store.pool.close().await;
    let backups = std::fs::read_dir(&directory.0)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("termvexa.backup-")
        })
        .count();
    assert_eq!(backups, 1);
    // 已升级数据库再次打开时，不重复添加列，也不重新执行已应用迁移。
    let reopened = SqliteStore::open(&path).await.unwrap();
    reopened.pool.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn initializes_new_database_with_both_migrations() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("new.sqlite3"))
        .await
        .unwrap();
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&store.pool)
            .await
            .unwrap();
    assert_eq!(versions, vec![1, 2]);
    let fingerprint: Option<String> =
        sqlx::query_scalar("SELECT content_fingerprint FROM key_files LIMIT 1")
            .fetch_optional(&store.pool)
            .await
            .unwrap();
    assert!(fingerprint.is_none());
    store.pool.close().await;
}

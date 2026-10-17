use super::*;

struct TestJournal {
    root: PathBuf,
    journal: Journal,
}
impl TestJournal {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("termeta-rekey-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let journal = Journal::new(&root.join("vault.sqlite3"));
        std::fs::write(&journal.database, b"original-database").unwrap();
        std::fs::write(journal.metadata(), b"original-metadata").unwrap();
        Self { root, journal }
    }
    fn stage(&self) {
        // 文件状态机使用占位字节；真实 SQLCipher 改密另由异步回归覆盖。
        self.journal.create(b"next-metadata").unwrap();
        std::fs::write(self.journal.file("next.sqlite3"), b"next-database").unwrap();
    }
    fn assert_original(&self) {
        assert_eq!(
            std::fs::read(&self.journal.database).unwrap(),
            b"original-database"
        );
        assert_eq!(
            std::fs::read(self.journal.metadata()).unwrap(),
            b"original-metadata"
        );
    }
}
impl Drop for TestJournal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn every_interrupted_file_switch_restores_the_original_pair() {
    for completed in 0..=4 {
        let files = TestJournal::new();
        files.stage();
        let journal = &files.journal;
        journal.mark("prepared").unwrap();
        let moves = [
            (journal.database.clone(), journal.file("displaced.sqlite3")),
            (journal.file("next.sqlite3"), journal.database.clone()),
            (journal.metadata(), journal.file("displaced.vault.json")),
            (journal.file("next.vault.json"), journal.metadata()),
        ];
        for (from, to) in moves.into_iter().take(completed) {
            std::fs::rename(from, to).unwrap();
        }
        assert!(!journal.recover().unwrap());
        files.assert_original();
        assert!(journal.file("original.sqlite3").exists());
        journal.cleanup().unwrap();
        assert!(!journal.directory.exists());
        journal.recover().unwrap();
        files.assert_original();
    }
}

#[test]
fn interrupted_staging_and_marker_write_do_not_replace_the_live_pair() {
    let files = TestJournal::new();
    files.stage();
    std::fs::write(files.journal.file("prepared.tmp"), b"partial").unwrap();
    assert!(!files.journal.recover().unwrap());
    files.journal.cleanup().unwrap();
    files.assert_original();
}

#[test]
fn interrupted_restore_can_be_repeated_before_its_terminal_marker() {
    let files = TestJournal::new();
    files.stage();
    let journal = &files.journal;
    journal.mark("prepared").unwrap();
    journal.install().unwrap();
    remove_file(&journal.database).unwrap();
    copy_new(&journal.file("original.sqlite3"), &journal.database).unwrap();
    journal.recover().unwrap();
    files.assert_original();
}

#[test]
fn terminal_markers_prevent_cleanup_retry_from_losing_later_writes() {
    for committed in [false, true] {
        let files = TestJournal::new();
        files.stage();
        let journal = &files.journal;
        journal.mark("prepared").unwrap();
        journal.install().unwrap();
        if committed {
            journal.mark("committed").unwrap();
        }
        // 在固定临时文件名处制造清理错误，终态标记必须留存以保护恢复后的新写入。
        std::fs::create_dir(journal.file("next.sqlite3-shm")).unwrap();
        assert_eq!(journal.recover().unwrap(), committed);
        assert!(journal.cleanup().is_err());
        assert!(
            journal
                .marked(if committed {
                    "committed"
                } else {
                    "rolled-back"
                })
                .unwrap()
        );
        std::fs::write(&journal.database, b"later-database-write").unwrap();
        std::fs::write(journal.metadata(), b"later-metadata-write").unwrap();
        std::fs::remove_dir(journal.file("next.sqlite3-shm")).unwrap();
        assert_eq!(journal.recover().unwrap(), committed);
        journal.cleanup().unwrap();
        assert_eq!(
            std::fs::read(&journal.database).unwrap(),
            b"later-database-write"
        );
        assert_eq!(
            std::fs::read(journal.metadata()).unwrap(),
            b"later-metadata-write"
        );
    }
}

#[test]
fn damaged_marker_fails_closed_and_preserves_all_files() {
    let files = TestJournal::new();
    files.stage();
    std::fs::write(files.journal.file("prepared"), b"invalid").unwrap();
    assert!(files.journal.recover().is_err());
    files.assert_original();
    assert!(files.journal.file("original.sqlite3").exists());
}

#[tokio::test(flavor = "current_thread")]
async fn rekey_preserves_wal_data_migrations_and_session_logs() {
    let files = TestJournal::new();
    remove_file(&files.journal.database).unwrap();
    let store = SqliteStore::open(&files.journal.database).await.unwrap();
    store
        .set_setting("test", 1, &serde_json::json!({"preserved": true}))
        .await
        .unwrap();
    let log_id = uuid::Uuid::new_v4().to_string();
    store
        .save_session_log(SessionLogDraft {
            id: log_id.clone(),
            protocol: "local".into(),
            name: "本地终端".into(),
            username: None,
            host: None,
            port: None,
            started_at: 1,
            content: "已显示的日志\n".into(),
        })
        .await
        .unwrap();
    let before: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT version,checksum FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert!(
        !store
            .change_key(&[42; 32], &[43; 32], b"next-metadata".to_vec())
            .await
            .unwrap()
            .cleanup_pending
    );
    assert_eq!(
        store.get_setting("test").await.unwrap().unwrap().value,
        serde_json::json!({"preserved": true})
    );
    let after: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT version,checksum FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        store.read_session_log(&log_id).await.unwrap().content,
        "已显示的日志\n"
    );
    store.delete_session_log(&log_id).await.unwrap();
    store.pool().await.unwrap().close().await;
    assert!(
        SqliteStore::checked_pool(&store.path, &[42; 32], SqliteJournalMode::Wal)
            .await
            .is_err()
    );
    let reopened = SqliteStore::checked_pool(&store.path, &[43; 32], SqliteJournalMode::Wal)
        .await
        .unwrap();
    reopened.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn waiting_database_write_resumes_on_the_new_pool() {
    let files = TestJournal::new();
    remove_file(&files.journal.database).unwrap();
    let store = std::sync::Arc::new(SqliteStore::open(&files.journal.database).await.unwrap());
    let held = store.pool().await.unwrap();
    let rotating = store.clone();
    let rotation = tokio::spawn(async move {
        rotating
            .change_key(&[42; 32], &[43; 32], b"next-metadata".to_vec())
            .await
    });
    tokio::task::yield_now().await;
    let writing = store.clone();
    let write = tokio::spawn(async move {
        writing
            .set_setting("queued", 1, &serde_json::json!(true))
            .await
    });
    tokio::task::yield_now().await;
    assert!(!rotation.is_finished());
    assert!(!write.is_finished());
    drop(held);
    rotation.await.unwrap().unwrap();
    write.await.unwrap().unwrap();
    assert_eq!(
        store.get_setting("queued").await.unwrap().unwrap().value,
        serde_json::json!(true)
    );
    store.pool().await.unwrap().close().await;
}

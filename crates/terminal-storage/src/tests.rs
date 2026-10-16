use super::*;
use std::path::PathBuf;
use terminal_core::AuthStep;

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("termvexa-storage-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn draft() -> HostDraft {
    HostDraft {
        id: None,
        revision: None,
        name: "测试主机".into(),
        address: "example.com".into(),
        port: 22,
        username: "user".into(),
        group_id: None,
        favorite: false,
        tags: vec![],
        auth_plan: vec![AuthStep::Password {
            credential_id: None,
        }],
    }
}
fn edit(host: HostProfile) -> HostDraft {
    HostDraft {
        id: Some(host.id),
        revision: Some(host.revision),
        name: host.name,
        address: host.address,
        port: host.port,
        username: host.username,
        group_id: host.group_id,
        favorite: host.favorite,
        tags: host.tags,
        auth_plan: host.auth_plan,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn initializes_rebuilt_schema_with_password_and_private_key_credentials() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("new.sqlite3"))
        .await
        .unwrap();
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(store.pool().unwrap())
            .await
            .unwrap();
    assert_eq!(versions, vec![1]);
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(store.pool().unwrap())
        .await
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    assert!(
        store
            .begin_credential("passphrase", "旧口令类型")
            .await
            .is_err()
    );
    let key = store.begin_credential("privateKey", "私钥").await.unwrap();
    assert_eq!(store.get_credential(&key).await.unwrap().kind, "privateKey");
    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('profile_auth_steps')")
            .fetch_all(store.pool().unwrap())
            .await
            .unwrap();
    assert!(!columns.contains(&"key_file_id".to_string()));
    assert!(!columns.contains(&"passphrase_credential_id".to_string()));
    store.pool().unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn validates_public_credential_type_and_prevents_deleting_bound_credential() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("new.sqlite3"))
        .await
        .unwrap();
    let key = store.begin_credential("privateKey", "私钥").await.unwrap();
    let mut host = draft();
    host.auth_plan = vec![AuthStep::PrivateKey {
        credential_id: key.clone(),
    }];
    assert!(store.save_host(host.clone()).await.is_err());
    store.set_credential_secret(&key, "测试秘密").await.unwrap();
    host.auth_plan = vec![AuthStep::Password {
        credential_id: Some(key.clone()),
    }];
    assert!(store.save_host(host.clone()).await.is_err());
    host.auth_plan = vec![AuthStep::PrivateKey {
        credential_id: key.clone(),
    }];
    let saved = store.save_host(host).await.unwrap();
    assert!(!saved.has_password);
    assert!(store.begin_delete_credential(&key).await.is_err());
    let duplicate = store.duplicate_host(&saved.id).await.unwrap();
    assert!(
        matches!(duplicate.auth_plan.as_slice(), [AuthStep::PrivateKey { credential_id }] if credential_id == &key)
    );
    store.pool().unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn host_password_is_hidden_owned_preserved_and_explicitly_cleared() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("new.sqlite3"))
        .await
        .unwrap();
    let owner = uuid::Uuid::new_v4().to_string();
    let secret = store.begin_host_password(&owner, "主机密码").await.unwrap();
    store
        .set_credential_secret(&secret, "测试秘密")
        .await
        .unwrap();
    let saved = store
        .save_host_with_password_ref(draft(), Some(&secret))
        .await
        .unwrap();
    assert_eq!(saved.id, owner);
    assert!(saved.has_password);
    assert!(matches!(
        saved.auth_plan.as_slice(),
        [AuthStep::Password {
            credential_id: None
        }]
    ));
    assert!(store.list_credentials().await.unwrap().is_empty());
    assert_eq!(store.list_all_credentials().await.unwrap().len(), 1);
    let config = store.get_host_connection(&owner).await.unwrap();
    assert!(
        matches!(config.auth_plan.as_slice(), [AuthStep::Password { credential_id: Some(reference) }] if reference == &secret)
    );
    let duplicate = store.duplicate_host(&owner).await.unwrap();
    assert!(!duplicate.has_password);
    assert!(
        store
            .save_host_with_password_ref(edit(duplicate.clone()), Some(&secret))
            .await
            .is_err()
    );
    let mut forged = edit(duplicate);
    forged.auth_plan = vec![AuthStep::Password {
        credential_id: Some(secret.clone()),
    }];
    assert!(store.save_host(forged).await.is_err());
    let preserved = store.save_host(edit(saved)).await.unwrap();
    assert!(preserved.has_password);
    let cleared = store
        .save_host_with_password_ref(edit(preserved), None)
        .await
        .unwrap();
    assert!(!cleared.has_password);
    assert_eq!(
        store.get_credential(&secret).await.unwrap().status,
        "deleting"
    );
    store.finish_delete_credential(&secret).await.unwrap();
    assert!(matches!(
        store.get_credential(&secret).await,
        Err(StorageError::NotFound)
    ));
    store.pool().unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn replacing_and_deleting_host_marks_only_owned_password_for_cleanup() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("new.sqlite3"))
        .await
        .unwrap();
    let owner = uuid::Uuid::new_v4().to_string();
    let first = store.begin_host_password(&owner, "原密码").await.unwrap();
    store
        .set_credential_secret(&first, "测试秘密")
        .await
        .unwrap();
    let host = store
        .save_host_with_password_ref(draft(), Some(&first))
        .await
        .unwrap();
    let second = store.begin_host_password(&owner, "新密码").await.unwrap();
    store
        .set_credential_secret(&second, "测试秘密")
        .await
        .unwrap();
    let host = store
        .save_host_with_password_ref(edit(host), Some(&second))
        .await
        .unwrap();
    assert_eq!(
        store.get_credential(&first).await.unwrap().status,
        "deleting"
    );
    assert_eq!(store.get_credential(&second).await.unwrap().status, "ready");
    let public = store
        .begin_credential("password", "公共密码")
        .await
        .unwrap();
    store
        .set_credential_secret(&public, "测试秘密")
        .await
        .unwrap();
    let orphan = store
        .begin_host_password(&owner, "中断的保存")
        .await
        .unwrap();
    store
        .set_credential_secret(&orphan, "测试秘密")
        .await
        .unwrap();
    store
        .mark_orphaned_host_credentials_deleting()
        .await
        .unwrap();
    assert_eq!(
        store.get_credential(&orphan).await.unwrap().status,
        "deleting"
    );
    assert_eq!(store.get_credential(&public).await.unwrap().status, "ready");
    assert_eq!(store.get_credential(&second).await.unwrap().status, "ready");
    assert!(store.delete_host(&owner, host.revision + 1).await.is_err());
    assert_eq!(store.get_credential(&second).await.unwrap().status, "ready");
    store.delete_host(&owner, host.revision).await.unwrap();
    assert_eq!(
        store.get_credential(&second).await.unwrap().status,
        "deleting"
    );
    store.pool().unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn save_returns_committed_revision_and_normalized_tags() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("new.sqlite3"))
        .await
        .unwrap();
    let mut input = draft();
    input.name = "  正规名称  ".into();
    input.tags = vec![" z ".into(), "a".into(), "z".into()];
    let saved = store.save_host(input).await.unwrap();
    assert_eq!(saved.name, "正规名称");
    assert_eq!(saved.tags, vec!["a".to_string(), "z".to_string()]);
    assert_eq!(saved.revision, 1);
    assert!(!saved.has_password);
    let id = saved.id.clone();
    let saved = store.save_host(edit(saved)).await.unwrap();
    assert_eq!(saved.revision, 2);
    let revision: i64 = sqlx::query_scalar("SELECT revision FROM connection_profiles WHERE id=?")
        .bind(id)
        .fetch_one(store.pool().unwrap())
        .await
        .unwrap();
    assert_eq!(saved.revision, revision);
    store.pool().unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn encrypted_database_rejects_wrong_key_and_preserves_data() {
    let directory = TestDirectory::new();
    let path = directory.0.join("encrypted.sqlite3");
    let store = SqliteStore::locked(&path);
    assert!(matches!(
        store.list_hosts("").await,
        Err(StorageError::Locked)
    ));
    assert!(!path.exists());
    store.unlock(&[17; 32]).await.unwrap();
    let saved = store.save_host(draft()).await.unwrap();
    let id = store
        .begin_credential("password", "encrypted-label")
        .await
        .unwrap();
    store
        .set_credential_secret(&id, "needle-secret-test-only")
        .await
        .unwrap();
    store.pool().unwrap().close().await;
    let bytes = std::fs::read(&path).unwrap();
    assert!(!bytes.starts_with(b"SQLite format 3"));
    for needle in [
        b"example.com".as_slice(),
        b"needle-secret-test-only",
        b"encrypted-label",
    ] {
        assert!(!bytes.windows(needle.len()).any(|window| window == needle));
    }
    let wrong = SqliteStore::locked(&path);
    assert!(wrong.unlock(&[18; 32]).await.is_err());
    assert!(!wrong.is_unlocked());
    let reopened = SqliteStore::locked(&path);
    reopened.unlock(&[17; 32]).await.unwrap();
    assert_eq!(
        reopened.get_host(&saved.id).await.unwrap().address,
        "example.com"
    );
    assert_eq!(
        reopened.credential_secret(&id).await.unwrap().as_str(),
        "needle-secret-test-only"
    );
    reopened.pool().unwrap().close().await;
    let mut damaged = bytes;
    damaged[100] ^= 1;
    std::fs::write(&path, damaged).unwrap();
    let corrupt = SqliteStore::locked(&path);
    assert!(corrupt.unlock(&[17; 32]).await.is_err());
    assert!(!corrupt.is_unlocked());
}

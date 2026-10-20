use super::*;
#[test]
fn vault_command_futures_are_send() {
    fn assert_send(_: impl Send) {}

    let vault = CredentialVault::new(std::sync::Arc::new(terminal_storage::SqliteStore::locked(
        "unused-vault-send-check.sqlite3",
    )));
    // 只构造并丢弃 future，不轮询，不打开数据库或访问系统钥匙串。
    assert_send(vault.status());
    assert_send(vault.unlock_with_password(Zeroizing::new(String::new()), false));
    assert_send(
        vault.change_password(Zeroizing::new(String::new()), Zeroizing::new(String::new())),
    );
}

#[test]
fn password_recovery_rejects_wrong_password_and_tampering() {
    let (header, key) = Header::create("正确的保险库密码-test-123").unwrap();
    header.verify(&key).unwrap();
    let recovered = derive("正确的保险库密码-test-123", &header.salt).unwrap();
    assert_eq!(&*key, &*recovered);
    let wrong = derive("错误密码-test-456", &header.salt).unwrap();
    assert!(header.verify(&wrong).is_err());
    let mut changed = header.clone();
    changed.salt[0] ^= 1;
    assert!(changed.verify(&key).is_err());
    let mut changed = header.clone();
    changed.verifier[0] ^= 1;
    assert!(changed.verify(&key).is_err());
    let mut changed = header.clone();
    changed.initialized = true;
    assert!(changed.verify(&key).is_err());
    let (other, _) = Header::create("正确的保险库密码-test-123").unwrap();
    assert_ne!(header.salt, other.salt);
}

#[derive(Default)]
struct FakeKeychain {
    key: std::sync::Mutex<Option<Vec<u8>>>,
    reads: std::sync::atomic::AtomicUsize,
    writes: std::sync::atomic::AtomicUsize,
    fail_write: std::sync::atomic::AtomicBool,
}
impl MasterKeyBackend for FakeKeychain {
    fn load(&self) -> Result<Zeroizing<Vec<u8>>, SecurityError> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.key
            .lock()
            .unwrap()
            .clone()
            .map(Zeroizing::new)
            .ok_or(SecurityError::Unavailable)
    }
    fn save(&self, key: &[u8]) -> Result<(), SecurityError> {
        self.writes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.fail_write.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(SecurityError::Unavailable);
        }
        *self.key.lock().unwrap() = Some(key.to_vec());
        Ok(())
    }
}
struct TestPath(PathBuf);
impl TestPath {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("termeta-vault-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn vault(&self, backend: std::sync::Arc<FakeKeychain>) -> CredentialVault {
        let mut vault = CredentialVault::new(std::sync::Arc::new(
            terminal_storage::SqliteStore::locked(self.0.join("vault.sqlite3")),
        ));
        vault.key_backend = backend;
        vault
    }
}
impl Drop for TestPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn canceling_password_change_caller_does_not_interrupt_commit_or_cache_update() {
    use std::sync::{Arc, atomic::Ordering};
    let path = TestPath::new();
    let backend = Arc::new(FakeKeychain::default());
    let vault = path.vault(backend.clone());
    vault
        .unlock_with_password(Zeroizing::new("local-original-password".into()), true)
        .await
        .unwrap();
    // 阻塞实际文件切换，让调用方取消发生在后台任务已接管且提交尚未开始时。
    let operation = vault.operations.lock().await;
    let worker = vault.clone();
    let caller = tokio::spawn(async move {
        worker
            .change_password(
                Zeroizing::new("local-original-password".into()),
                Zeroizing::new("changed-vault-password".into()),
            )
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            if vault.boot.try_lock().is_err() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    drop(operation);
    // status 等待同一个协调锁，成功返回证明后台任务完成新密钥提交及缓存写入。
    let status = tokio::time::timeout(std::time::Duration::from_secs(30), vault.status())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.mode, "unlocked");
    let header = read_header(&vault.header_path()).unwrap().unwrap();
    let expected = derive("changed-vault-password", &header.salt).unwrap();
    header.verify(&expected).unwrap();
    assert_eq!(
        backend.key.lock().unwrap().as_deref(),
        Some(expected.as_slice())
    );
    assert_eq!(backend.writes.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn password_change_checks_old_password_and_preserves_credentials() {
    use std::sync::{Arc, atomic::Ordering};
    let path = TestPath::new();
    let backend = Arc::new(FakeKeychain::default());
    let vault = path.vault(backend.clone());
    vault
        .unlock_with_password(Zeroizing::new("original-password-123".into()), true)
        .await
        .unwrap();
    let credential = vault
        .save_password("保留的凭据", Zeroizing::new("stored-secret".into()))
        .await
        .unwrap();
    let before = std::fs::read(vault.header_path()).unwrap();
    let database_before = std::fs::read(vault.store.path()).unwrap();
    assert!(
        vault
            .change_password(
                Zeroizing::new("wrong-password".into()),
                Zeroizing::new("new-password-456".into())
            )
            .await
            .is_err()
    );
    assert!(
        vault
            .change_password(
                Zeroizing::new("original-password-123".into()),
                Zeroizing::new("short".into())
            )
            .await
            .is_err()
    );
    assert!(
        vault
            .change_password(
                Zeroizing::new("original-password-123".into()),
                Zeroizing::new("original-password-123".into())
            )
            .await
            .is_err()
    );
    assert_eq!(before, std::fs::read(vault.header_path()).unwrap());
    assert_eq!(database_before, std::fs::read(vault.store.path()).unwrap());
    assert_eq!(backend.writes.load(Ordering::SeqCst), 1);
    let original = read_header(&vault.header_path()).unwrap().unwrap();
    let changed = vault
        .change_password(
            Zeroizing::new("original-password-123".into()),
            Zeroizing::new("  新的保险库密码-password-456  ".into()),
        )
        .await
        .unwrap();
    assert!(changed.keychain_warning.is_none());
    assert!(changed.cleanup_warning.is_none());
    let next = read_header(&vault.header_path()).unwrap().unwrap();
    assert_eq!(next.id, original.id);
    assert_ne!(next.salt, original.salt);
    assert_ne!(next.nonce, original.nonce);
    assert_eq!(
        vault.load(&credential).await.unwrap().as_str(),
        "stored-secret"
    );
    assert_eq!(backend.writes.load(Ordering::SeqCst), 2);
    let automatic = path.vault(backend.clone());
    assert_eq!(automatic.status().await.unwrap().mode, "unlocked");
    *backend.key.lock().unwrap() = None;
    let recovered = path.vault(backend);
    assert_eq!(recovered.status().await.unwrap().mode, "locked");
    assert!(
        recovered
            .unlock_with_password(Zeroizing::new("original-password-123".into()), false)
            .await
            .is_err()
    );
    assert!(
        recovered
            .unlock_with_password(Zeroizing::new("新的保险库密码-password-456".into()), false)
            .await
            .is_err()
    );
    recovered
        .unlock_with_password(
            Zeroizing::new("  新的保险库密码-password-456  ".into()),
            false,
        )
        .await
        .unwrap();
    assert_eq!(
        recovered.load(&credential).await.unwrap().as_str(),
        "stored-secret"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn changed_password_survives_keychain_write_failure() {
    use std::sync::{Arc, atomic::Ordering};
    let path = TestPath::new();
    let backend = Arc::new(FakeKeychain::default());
    let vault = path.vault(backend.clone());
    vault
        .unlock_with_password(Zeroizing::new("original-password-123".into()), true)
        .await
        .unwrap();
    let old_cached_key = backend.key.lock().unwrap().clone();
    backend.fail_write.store(true, Ordering::SeqCst);
    let changed = vault
        .change_password(
            Zeroizing::new("original-password-123".into()),
            Zeroizing::new("new-password-456".into()),
        )
        .await
        .unwrap();
    assert!(changed.keychain_warning.is_some());
    assert_eq!(old_cached_key, *backend.key.lock().unwrap());
    let header = read_header(&vault.header_path()).unwrap().unwrap();
    let current = derive("new-password-456", &header.salt).unwrap();
    header.verify(&current).unwrap();
    let recovered = path.vault(backend);
    assert_eq!(recovered.status().await.unwrap().mode, "locked");
    assert!(
        recovered
            .unlock_with_password(Zeroizing::new("original-password-123".into()), false)
            .await
            .is_err()
    );
    assert_eq!(
        recovered
            .unlock_with_password(Zeroizing::new("new-password-456".into()), false)
            .await
            .unwrap()
            .mode,
        "unlocked"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn concurrent_changes_cannot_overwrite_a_newer_password() {
    let path = TestPath::new();
    let backend = std::sync::Arc::new(FakeKeychain::default());
    let vault = path.vault(backend.clone());
    vault
        .unlock_with_password(Zeroizing::new("original-password-123".into()), true)
        .await
        .unwrap();
    let (first, second) = tokio::join!(
        vault.change_password(
            Zeroizing::new("original-password-123".into()),
            Zeroizing::new("first-new-password".into())
        ),
        vault.change_password(
            Zeroizing::new("original-password-123".into()),
            Zeroizing::new("second-new-password".into())
        ),
    );
    assert_ne!(first.is_ok(), second.is_ok());
    let winner = if first.is_ok() {
        "first-new-password"
    } else {
        "second-new-password"
    };
    *backend.key.lock().unwrap() = None;
    let recovered = path.vault(backend);
    recovered
        .unlock_with_password(Zeroizing::new(winner.into()), false)
        .await
        .unwrap();
}
#[tokio::test(flavor = "current_thread")]
async fn one_keychain_item_and_password_recovery() {
    use std::sync::{Arc, atomic::Ordering};
    let path = TestPath::new();
    let backend = Arc::new(FakeKeychain::default());
    let first = path.vault(backend.clone());
    assert_eq!(first.status().await.unwrap().mode, "setup");
    assert_eq!(
        first
            .unlock_with_password(Zeroizing::new("test-master-password".into()), true)
            .await
            .unwrap()
            .mode,
        "unlocked"
    );
    let credential = first
        .save_password("first", Zeroizing::new("secret".into()))
        .await
        .unwrap();
    first.load(&credential).await.unwrap();
    assert_eq!(backend.reads.load(Ordering::SeqCst), 0);
    assert_eq!(backend.writes.load(Ordering::SeqCst), 1);
    let second = path.vault(backend.clone());
    assert_eq!(second.status().await.unwrap().mode, "unlocked");
    second.status().await.unwrap();
    assert_eq!(backend.reads.load(Ordering::SeqCst), 1);
    *backend.key.lock().unwrap() = Some(vec![99; 32]);
    let third = path.vault(backend.clone());
    assert_eq!(third.status().await.unwrap().mode, "locked");
    third.status().await.unwrap();
    assert_eq!(backend.reads.load(Ordering::SeqCst), 2);
    assert!(
        third
            .unlock_with_password(Zeroizing::new("wrong-password".into()), false)
            .await
            .is_err()
    );
    assert!(!third.is_unlocked());
    assert_eq!(backend.writes.load(Ordering::SeqCst), 1);
    assert_eq!(
        third
            .unlock_with_password(Zeroizing::new("test-master-password".into()), false)
            .await
            .unwrap()
            .mode,
        "unlocked"
    );
    assert_eq!(third.load(&credential).await.unwrap().as_str(), "secret");
    assert_eq!(backend.writes.load(Ordering::SeqCst), 2);
}
#[tokio::test(flavor = "current_thread")]
async fn keychain_write_failure_does_not_lose_password_recovery() {
    use std::sync::{Arc, atomic::Ordering};
    let path = TestPath::new();
    let backend = Arc::new(FakeKeychain::default());
    backend.fail_write.store(true, Ordering::SeqCst);
    let first = path.vault(backend.clone());
    let status = first
        .unlock_with_password(Zeroizing::new("test-master-password".into()), true)
        .await
        .unwrap();
    assert_eq!(status.mode, "unlocked");
    assert!(status.keychain_warning.is_some());
    let next = path.vault(backend.clone());
    assert_eq!(next.status().await.unwrap().mode, "locked");
    assert_eq!(
        next.unlock_with_password(Zeroizing::new("test-master-password".into()), false)
            .await
            .unwrap()
            .mode,
        "unlocked"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn interrupted_initialization_recovers_but_truncated_database_does_not_reset() {
    let path = TestPath::new();
    let backend = std::sync::Arc::new(FakeKeychain::default());
    let vault = path.vault(backend.clone());
    let (header, key) = Header::create("test-master-password").unwrap();
    write_header(&vault.header_path(), &header, false).unwrap();
    assert_eq!(vault.status().await.unwrap().mode, "locked");
    vault
        .unlock_with_password(Zeroizing::new("test-master-password".into()), false)
        .await
        .unwrap();
    let completed = read_header(&vault.header_path()).unwrap().unwrap();
    assert!(completed.initialized);
    assert_ne!(completed.nonce, header.nonce);
    completed.verify(&key).unwrap();
    // 用独立缺失数据库场景验证完成态保护，不截断仍由活动池打开的文件。
    let missing = TestPath::new();
    let next = missing.vault(backend);
    write_header(&next.header_path(), &completed, false).unwrap();
    std::fs::write(next.store.path(), []).unwrap();
    assert_eq!(next.status().await.unwrap().mode, "locked");
    assert!(
        next.unlock_with_password(Zeroizing::new("test-master-password".into()), false)
            .await
            .is_err()
    );
    assert!(!next.is_unlocked());
    assert_eq!(std::fs::metadata(next.store.path()).unwrap().len(), 0);
}

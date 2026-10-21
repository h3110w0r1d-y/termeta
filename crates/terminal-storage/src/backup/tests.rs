use super::*;
use crate::{HostDraft, SavedLayout, SavedWorkspace, WorkspaceDraft};
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use sha2::{Digest, Sha256};
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
    let snippet = source
        .save_snippet(crate::SnippetDraft {
            id: None,
            revision: None,
            name: "查看状态".into(),
            content: "systemctl status\n".into(),
        })
        .await
        .unwrap();
    source
        .save_session_log(crate::SessionLogDraft {
            id: uuid::Uuid::new_v4().to_string(),
            protocol: "local".into(),
            name: "本机日志".into(),
            username: None,
            host: None,
            port: None,
            started_at: 0,
            content: "private-session-log-do-not-export".into(),
        })
        .await
        .unwrap();
    let snapshot = source.export_backup_database().await.unwrap();
    assert!(snapshot.starts_with(b"SQLite format 3\0"));
    assert!(
        !snapshot
            .windows(b"private-session-log-do-not-export".len())
            .any(|bytes| bytes == b"private-session-log-do-not-export")
    );
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
    assert_eq!(summary.counts.snippets, 1);
    assert!(target.list_session_logs().await.unwrap().is_empty());
    let imported_snippets = target.list_snippets().await.unwrap();
    assert_eq!(imported_snippets[0].content, snippet.content);
    assert_ne!(imported_snippets[0].id, snippet.id);
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
fn previous_snapshot_without_snippets_is_still_accepted() {
    let data: Data = TABLES[..9].iter().map(|_| Vec::new()).collect();
    let mut bytes = crate::backup_sqlite::encode(&data).unwrap();
    // SQLite 文件头的 user_version 为大端，旧快照使用版本 1。
    bytes[60..64].copy_from_slice(&1u32.to_be_bytes());
    assert_eq!(
        inspect_backup_database(bytes.to_vec()).unwrap(),
        BackupCounts::default()
    );
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

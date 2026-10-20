use super::*;
use std::path::PathBuf;
use terminal_core::AuthStep;

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("termeta-storage-{}", uuid::Uuid::new_v4()));
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
        startup: terminal_core::SshStartup::default(),
        kind: "ssh".into(),
        serial_config: None,
        icon: "ssh".into(),
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
        startup: host.startup,
        kind: host.kind,
        serial_config: host.serial_config,
        icon: host.icon,
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

fn serial_draft() -> HostDraft {
    let mut host = draft();
    host.kind = "serial".into();
    host.name = "离线串口配置".into();
    host.address.clear();
    host.port = 0;
    host.username.clear();
    host.auth_plan.clear();
    host.serial_config = Some(terminal_core::SerialConfig {
        path: "/dev/termeta-not-connected".into(),
        baud_rate: 115200,
        charset: "UTF-8".into(),
        data_bits: 8,
        stop_bits: 1,
        flow_control: terminal_core::SerialFlowControl::None,
        parity: terminal_core::SerialParity::None,
    });
    host
}

#[tokio::test(flavor = "current_thread")]
async fn forward_rules_reference_ssh_and_survive_host_deletion() {
    use terminal_core::{ForwardMode, ForwardRule};
    let directory = TestDirectory::new();
    let path = directory.0.join("forwarding.sqlite3");
    let store = SqliteStore::open(&path).await.unwrap();
    let ssh = store.save_host(draft()).await.unwrap();
    let serial = store.save_host(serial_draft()).await.unwrap();
    let mut draft = ForwardDraft {
        id: None,
        name: "数据库".into(),
        profile_id: ssh.id.clone(),
        rule: ForwardRule {
            mode: ForwardMode::Local,
            bind_address: "127.0.0.1".into(),
            bind_port: 15432,
            target_address: Some("db.internal".into()),
            target_port: Some(5432),
        },
    };
    let first = store.save_forward(draft.clone(), None).await.unwrap();
    draft.id = Some(first.id.clone());
    draft.name = "修改后".into();
    let edited = store
        .save_forward(draft.clone(), Some(first.revision))
        .await
        .unwrap();
    assert!(matches!(
        store
            .save_forward(draft.clone(), Some(first.revision))
            .await,
        Err(StorageError::Conflict)
    ));
    assert!(matches!(
        store.delete_forward(&first.id, first.revision).await,
        Err(StorageError::Conflict)
    ));
    draft.id = None;
    draft.profile_id = serial.id;
    assert!(store.save_forward(draft.clone(), None).await.is_err());
    draft.profile_id = ssh.id.clone();
    for mode in [ForwardMode::Remote, ForwardMode::Dynamic] {
        draft.rule.mode = mode;
        if mode == ForwardMode::Dynamic {
            draft.rule.target_address = None;
            draft.rule.target_port = None;
        }
        store.save_forward(draft.clone(), None).await.unwrap();
    }
    store.delete_host(&ssh.id, ssh.revision).await.unwrap();
    let profiles = store.list_forwards().await.unwrap();
    assert_eq!(profiles.len(), 3);
    assert!(profiles.iter().all(|p| p.profile_id.is_none()));
    assert!(store.get_host_connection(&ssh.id).await.is_err());
    store.pool().await.unwrap().close().await;
    let reopened = SqliteStore::open(&path).await.unwrap();
    assert_eq!(reopened.list_forwards().await.unwrap().len(), 3);
    reopened
        .delete_forward(&edited.id, edited.revision)
        .await
        .unwrap();
    assert_eq!(reopened.list_forwards().await.unwrap().len(), 2);
    reopened.pool().await.unwrap().close().await;
}

#[test]
fn legacy_host_dto_defaults_to_ssh() {
    let mut value = serde_json::to_value(draft()).unwrap();
    value.as_object_mut().unwrap().remove("kind");
    value.as_object_mut().unwrap().remove("serialConfig");
    value.as_object_mut().unwrap().remove("startup");
    let host: HostDraft = serde_json::from_value(value).unwrap();
    assert_eq!(host.kind, "ssh");
    assert!(host.serial_config.is_none());
    assert_eq!(host.startup, terminal_core::SshStartup::Shell);
}

#[tokio::test(flavor = "current_thread")]
async fn ssh_startup_roundtrips_and_legacy_records_keep_shell() {
    use terminal_core::SshStartup;
    let directory = TestDirectory::new();
    let path = directory.0.join("ssh-startup.sqlite3");
    let store = SqliteStore::open(&path).await.unwrap();
    let mut host = draft();
    host.startup = SshStartup::Exec {
        command: "  /usr/bin/btop --utf-force  ".into(),
    };
    let saved = store.save_host(host.clone()).await.unwrap();
    assert_eq!(saved.startup, host.startup);
    assert_eq!(
        store.get_host_connection(&saved.id).await.unwrap().startup,
        host.startup
    );
    let mut metadata = edit(saved.clone());
    metadata.favorite = true;
    let edited = store.save_host(metadata).await.unwrap();
    assert_eq!(edited.startup, host.startup);
    let copy = store.duplicate_host(&saved.id).await.unwrap();
    assert_eq!(copy.startup, host.startup);
    store.pool().await.unwrap().close().await;
    let store = SqliteStore::open(&path).await.unwrap();
    assert_eq!(
        store.get_host(&saved.id).await.unwrap().startup,
        host.startup
    );
    // 旧记录只缺少启动字段；读取时默认 Shell，不要求结构迁移。
    sqlx::query("UPDATE connection_profiles SET config_json=json_remove(config_json,'$.startup') WHERE id=?")
        .bind(&saved.id).execute(&*store.pool().await.unwrap()).await.unwrap();
    assert_eq!(
        store.get_host_connection(&saved.id).await.unwrap().startup,
        SshStartup::Shell
    );
    let mut shell = edit(store.get_host(&copy.id).await.unwrap());
    shell.startup = SshStartup::Shell;
    let shell = store.save_host(shell).await.unwrap();
    assert_eq!(
        store.get_host_connection(&shell.id).await.unwrap().startup,
        SshStartup::Shell
    );
    store.pool().await.unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_or_non_ssh_startup_never_falls_back_to_shell() {
    use terminal_core::SshStartup;
    let directory = TestDirectory::new();
    let store = SqliteStore::open(&directory.0.join("invalid-startup.sqlite3"))
        .await
        .unwrap();
    for command in ["", " \t", "btop\0suffix", &"中".repeat(2731)] {
        let mut host = draft();
        host.startup = SshStartup::Exec {
            command: command.into(),
        };
        assert!(store.save_host(host).await.is_err());
    }
    for mut host in [serial_draft(), telnet_draft()] {
        host.startup = SshStartup::Exec {
            command: "btop".into(),
        };
        assert!(store.save_host(host).await.is_err());
    }
    let saved = store.save_host(draft()).await.unwrap();
    for startup in [
        serde_json::json!(null),
        serde_json::json!({"type":"unknown"}),
        serde_json::json!({"type":"exec"}),
        serde_json::json!({"type":"exec","command":""}),
    ] {
        sqlx::query("UPDATE connection_profiles SET config_json=json_set(config_json,'$.startup',json(?)) WHERE id=?")
            .bind(startup.to_string()).bind(&saved.id).execute(&*store.pool().await.unwrap()).await.unwrap();
        assert!(store.get_host_connection(&saved.id).await.is_err());
        assert!(store.list_hosts("").await.is_err());
    }
    store.pool().await.unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn serial_hosts_persist_offline_and_preserve_configuration_across_metadata_edits() {
    use terminal_core::RemoteSystemStore;
    let directory = TestDirectory::new();
    let path = directory.0.join("serial.sqlite3");
    let store = SqliteStore::open(&path).await.unwrap();
    let ssh = store.save_host(draft()).await.unwrap();
    let saved = store.save_host(serial_draft()).await.unwrap();
    assert_eq!(saved.kind, "serial");
    assert_eq!(saved.port, 0);
    assert!(saved.address.is_empty());
    assert!(saved.auth_plan.is_empty());
    assert!(!saved.has_password);
    assert_eq!(store.list_hosts("").await.unwrap().len(), 2);
    assert_eq!(
        store
            .list_hosts("termeta-not-connected")
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(store.get_host_connection(&saved.id).await.is_err());
    let ssh_config = store.get_host_connection(&ssh.id).await.unwrap();
    assert!(!store.claim(&saved.id, "serial", &ssh_config).await.unwrap());
    let ssh_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM ssh_profiles WHERE profile_id=?")
        .bind(&saved.id)
        .fetch_one(&*store.pool().await.unwrap())
        .await
        .unwrap();
    assert_eq!(ssh_rows, 0);
    sqlx::query("UPDATE connection_profiles SET config_json=json_set(config_json,'$.futureMetadata','kept') WHERE id=?")
        .bind(&saved.id).execute(&*store.pool().await.unwrap()).await.unwrap();
    let group = store
        .save_group(GroupDraft {
            id: None,
            revision: None,
            parent_id: None,
            name: "设备".into(),
            sort_order: 0,
        })
        .await
        .unwrap();
    let serial_config = serde_json::to_value(&saved.serial_config).unwrap();
    let mut changed = edit(saved);
    changed.favorite = true;
    changed.group_id = Some(group.id.clone());
    changed.tags = vec!["开发板".into()];
    let saved = store.save_host(changed).await.unwrap();
    assert_eq!(
        serde_json::to_value(&saved.serial_config).unwrap(),
        serial_config
    );
    let metadata: String =
        sqlx::query_scalar("SELECT config_json FROM connection_profiles WHERE id=?")
            .bind(&saved.id)
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&metadata).unwrap()["futureMetadata"],
        "kept"
    );
    let duplicate = store.duplicate_host(&saved.id).await.unwrap();
    assert_eq!(duplicate.kind, "serial");
    assert_eq!(
        serde_json::to_value(&duplicate.serial_config).unwrap(),
        serial_config
    );
    assert!(duplicate.favorite);
    assert_eq!(duplicate.tags, saved.tags);
    assert_eq!(duplicate.group_id, saved.group_id);
    store.pool().await.unwrap().close().await;
    let store = SqliteStore::open(&path).await.unwrap();
    let restored = store.get_host(&saved.id).await.unwrap();
    assert_eq!(
        serde_json::to_value(&restored.serial_config).unwrap(),
        serial_config
    );
    store
        .delete_host(&restored.id, restored.revision)
        .await
        .unwrap();
    assert!(store.get_host(&restored.id).await.is_err());
    assert!(store.get_host(&duplicate.id).await.is_ok());
    store.pool().await.unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn host_kinds_reject_mixed_configuration_credentials_and_type_changes() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("validation.sqlite3"))
        .await
        .unwrap();
    let mut host = serial_draft();
    host.auth_plan = draft().auth_plan;
    assert!(store.save_host(host).await.is_err());
    let mut host = serial_draft();
    host.address = "example.com".into();
    assert!(store.save_host(host).await.is_err());
    let mut host = serial_draft();
    host.serial_config = None;
    assert!(store.save_host(host).await.is_err());
    let mut host = serial_draft();
    host.serial_config.as_mut().unwrap().baud_rate = 0;
    assert!(store.save_host(host).await.is_err());
    assert!(
        store
            .save_host_with_password_ref(serial_draft(), Some("forged"))
            .await
            .is_err()
    );
    let mut host = draft();
    host.serial_config = serial_draft().serial_config;
    assert!(store.save_host(host).await.is_err());
    let mut host = draft();
    host.kind = "telnet".into();
    assert!(store.save_host(host).await.is_err());
    let saved = store.save_host(draft()).await.unwrap();
    let mut host = serial_draft();
    host.id = Some(saved.id.clone());
    host.revision = Some(saved.revision);
    assert!(store.save_host(host).await.is_err());
    assert_eq!(store.get_host(&saved.id).await.unwrap().kind, "ssh");
    let saved = store.save_host(serial_draft()).await.unwrap();
    let mut host = draft();
    host.id = Some(saved.id.clone());
    host.revision = Some(saved.revision);
    assert!(store.save_host(host).await.is_err());
    assert_eq!(store.get_host(&saved.id).await.unwrap().kind, "serial");
    store.pool().await.unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn initializes_rebuilt_schema_with_password_and_private_key_credentials() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("new.sqlite3"))
        .await
        .unwrap();
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(versions, vec![1, 2, 3, 4, 5, 6]);
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&*store.pool().await.unwrap())
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
            .fetch_all(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert!(!columns.contains(&"key_file_id".to_string()));
    assert!(!columns.contains(&"passphrase_credential_id".to_string()));
    store.pool().await.unwrap().close().await;
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
    store.pool().await.unwrap().close().await;
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
    store.pool().await.unwrap().close().await;
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
    store.pool().await.unwrap().close().await;
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
        .fetch_one(&*store.pool().await.unwrap())
        .await
        .unwrap();
    assert_eq!(saved.revision, revision);
    store.pool().await.unwrap().close().await;
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
    store.pool().await.unwrap().close().await;
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
    reopened.pool().await.unwrap().close().await;
    let mut damaged = bytes;
    damaged[100] ^= 1;
    std::fs::write(&path, damaged).unwrap();
    let corrupt = SqliteStore::locked(&path);
    assert!(corrupt.unlock(&[17; 32]).await.is_err());
    assert!(!corrupt.is_unlocked());
}

async fn group_fixture(store: &SqliteStore, name: &str, parent_id: Option<String>) -> HostGroup {
    store
        .save_group(GroupDraft {
            id: None,
            revision: None,
            name: name.into(),
            parent_id,
            sort_order: 0,
        })
        .await
        .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn deleting_group_moves_only_direct_children_and_preserves_hosts() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("groups.sqlite3"))
        .await
        .unwrap();
    let parent = group_fixture(&store, "父组", None).await;
    let group = group_fixture(&store, "目标组", Some(parent.id.clone())).await;
    let child = group_fixture(&store, "子组", Some(group.id.clone())).await;
    let mut direct = draft();
    direct.group_id = Some(group.id.clone());
    let direct = store.save_host(direct).await.unwrap();
    let mut nested = draft();
    nested.name = "子组主机".into();
    nested.group_id = Some(child.id.clone());
    let nested = store.save_host(nested).await.unwrap();

    store.delete_group(&group.id, group.revision).await.unwrap();
    let groups = store.list_groups().await.unwrap();
    assert!(!groups.iter().any(|item| item.id == group.id));
    assert_eq!(
        groups
            .iter()
            .find(|item| item.id == child.id)
            .unwrap()
            .parent_id,
        Some(parent.id.clone())
    );
    assert_eq!(
        store.get_host(&direct.id).await.unwrap().group_id,
        Some(parent.id)
    );
    assert_eq!(
        store.get_host(&nested.id).await.unwrap().group_id,
        Some(child.id)
    );
    store.pool().await.unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn deleting_group_subtree_is_revision_guarded_and_preserves_shared_credentials() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("groups.sqlite3"))
        .await
        .unwrap();
    let parent = group_fixture(&store, "父组", None).await;
    let group = group_fixture(&store, "目标组", Some(parent.id.clone())).await;
    let child = group_fixture(&store, "子组", Some(group.id.clone())).await;
    let sibling = group_fixture(&store, "同级组", Some(parent.id.clone())).await;
    let unaffected = store.save_host(draft()).await.unwrap();
    let shared = store
        .begin_credential("password", "共享密码")
        .await
        .unwrap();
    store
        .set_credential_secret(&shared, "共享测试秘密")
        .await
        .unwrap();
    let mut public_host = draft();
    public_host.name = "公共凭据主机".into();
    public_host.group_id = Some(group.id.clone());
    public_host.auth_plan = vec![AuthStep::Password {
        credential_id: Some(shared.clone()),
    }];
    let public_host = store.save_host(public_host).await.unwrap();
    let owner = uuid::Uuid::new_v4().to_string();
    let private = store.begin_host_password(&owner, "专属密码").await.unwrap();
    store
        .set_credential_secret(&private, "专属测试秘密")
        .await
        .unwrap();
    let mut private_host = draft();
    private_host.name = "专属凭据主机".into();
    private_host.group_id = Some(child.id.clone());
    let private_host = store
        .save_host_with_password_ref(private_host, Some(&private))
        .await
        .unwrap();

    // 过期版本不能留下已删除主机或提前标记的秘密。
    assert!(matches!(
        store
            .delete_group_and_hosts(&group.id, group.revision + 1)
            .await,
        Err(StorageError::Conflict)
    ));
    assert!(store.get_host(&public_host.id).await.is_ok());
    assert_eq!(
        store.get_credential(&private).await.unwrap().status,
        "ready"
    );

    // 注入后置失败，证明主机删除和专属凭据状态也随整个事务回滚。
    sqlx::query("CREATE TRIGGER reject_group_delete BEFORE DELETE ON host_groups BEGIN SELECT RAISE(ABORT, '测试回滚'); END")
        .execute(&*store.pool().await.unwrap()).await.unwrap();
    assert!(
        store
            .delete_group_and_hosts(&group.id, group.revision)
            .await
            .is_err()
    );
    assert!(store.get_host(&private_host.id).await.is_ok());
    assert_eq!(
        store.get_credential(&private).await.unwrap().status,
        "ready"
    );
    sqlx::query("DROP TRIGGER reject_group_delete")
        .execute(&*store.pool().await.unwrap())
        .await
        .unwrap();

    store
        .delete_group_and_hosts(&group.id, group.revision)
        .await
        .unwrap();
    let groups = store.list_groups().await.unwrap();
    assert_eq!(groups.len(), 2);
    assert!(groups.iter().any(|item| item.id == parent.id));
    assert!(groups.iter().any(|item| item.id == sibling.id));
    assert!(matches!(
        store.get_host(&public_host.id).await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        store.get_host(&private_host.id).await,
        Err(StorageError::NotFound)
    ));
    assert!(store.get_host(&unaffected.id).await.is_ok());
    assert_eq!(store.get_credential(&shared).await.unwrap().status, "ready");
    assert_eq!(
        store.get_credential(&private).await.unwrap().status,
        "deleting"
    );
    assert_eq!(
        &*store.credential_secret(&shared).await.unwrap(),
        "共享测试秘密"
    );
    store.pool().await.unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn session_logs_keep_all_protocols_and_content_only_after_explicit_save() {
    let directory = TestDirectory::new();
    let path = directory.0.join("session-logs.sqlite3");
    let store = SqliteStore::open(&path).await.unwrap();
    let host = store.save_host(draft()).await.unwrap();
    assert!(store.list_session_logs().await.unwrap().is_empty());
    let mut saved_ids = Vec::new();
    for index in 0..25 {
        let protocol = ["ssh", "telnet", "serial", "local"][index % 4];
        let id = uuid::Uuid::new_v4().to_string();
        let content = format!("欢迎\n终端 {index}\n");
        let saved = store
            .save_session_log(SessionLogDraft {
                id: id.clone(),
                protocol: protocol.into(),
                name: "连接快照".into(),
                username: (protocol == "ssh").then(|| "root".into()),
                host: (protocol != "local").then(|| "example.com".into()),
                port: match protocol {
                    "ssh" => Some(22),
                    "telnet" => Some(23),
                    _ => None,
                },
                started_at: 1,
                content: content.clone(),
            })
            .await
            .unwrap();
        assert_eq!(saved.byte_length, content.len() as i64);
        assert_eq!(store.read_session_log(&id).await.unwrap().content, content);
        let mut retry = SessionLogDraft {
            id: id.clone(),
            protocol: protocol.into(),
            name: "最新连接快照".into(),
            username: None,
            host: None,
            port: None,
            started_at: 2,
            content: format!("{content}继续产生的输出\n"),
        };
        let repeated = store.save_session_log(retry.clone()).await.unwrap();
        assert!(repeated.closed_at >= saved.closed_at);
        assert_eq!(repeated.name, retry.name);
        assert_eq!(repeated.started_at, retry.started_at);
        assert_eq!(repeated.byte_length, retry.content.len() as i64);
        let latest = store.read_session_log(&id).await.unwrap();
        assert_eq!(latest.content, retry.content);
        assert_eq!(latest.summary.username, retry.username);
        assert_eq!(latest.summary.host, retry.host);
        assert_eq!(latest.summary.port, retry.port);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session_logs WHERE id=?")
            .bind(&id)
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
        assert_eq!(count, 1);
        retry.protocol = "workspace".into();
        assert!(matches!(
            store.save_session_log(retry).await,
            Err(StorageError::Validation(_))
        ));
        saved_ids.push(id);
    }
    store.delete_host(&host.id, host.revision).await.unwrap();
    assert_eq!(store.list_session_logs().await.unwrap().len(), 25);
    store.pool().await.unwrap().close().await;
    drop(store);
    let reopened = SqliteStore::open(&path).await.unwrap();
    assert_eq!(reopened.list_session_logs().await.unwrap().len(), 25);
    reopened.delete_session_log(&saved_ids[0]).await.unwrap();
    assert!(matches!(
        reopened.read_session_log(&saved_ids[0]).await,
        Err(StorageError::NotFound)
    ));
    reopened.clear_session_logs().await.unwrap();
    assert!(reopened.list_session_logs().await.unwrap().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn unlock_leaves_legacy_connection_history_untouched() {
    let directory = TestDirectory::new();
    let path = directory.0.join("legacy-history.sqlite3");
    let store = SqliteStore::open(&path).await.unwrap();
    for index in 0..25 {
        sqlx::query(
            "INSERT INTO session_history(id,display_name_snapshot,started_at) VALUES(?,?,?)",
        )
        .bind(format!("legacy-{index}"))
        .bind("旧连接")
        .bind(1_i64)
        .execute(&*store.pool().await.unwrap())
        .await
        .unwrap();
    }
    store.pool().await.unwrap().close().await;
    drop(store);
    let reopened = SqliteStore::open(&path).await.unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM session_history WHERE ended_at IS NULL")
            .fetch_one(&*reopened.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(count, 25);
    assert!(reopened.list_session_logs().await.unwrap().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn system_detection_failure_is_once_only_and_survives_reopen() {
    use terminal_core::RemoteSystemStore;
    let directory = TestDirectory::new();
    let path = directory.0.join("system.sqlite3");
    let store = SqliteStore::open(&path).await.unwrap();
    let host = store.save_host(draft()).await.unwrap();
    let config = store.get_host_connection(&host.id).await.unwrap();
    let (first, second) = tokio::join!(
        store.claim(&host.id, "first", &config),
        store.claim(&host.id, "second", &config),
    );
    let first = first.unwrap();
    assert_ne!(first, second.unwrap());
    let token = if first { "first" } else { "second" };
    store
        .complete(&host.id, token, &config, None)
        .await
        .unwrap();
    let failed = store.get_host(&host.id).await.unwrap();
    assert!(failed.system_detection_attempted);
    assert_eq!(failed.icon, "ssh");
    store.pool().await.unwrap().close().await;
    drop(store);
    let store = SqliteStore::open(&path).await.unwrap();
    assert!(!store.claim(&host.id, "reconnect", &config).await.unwrap());
    // 修改名称或地址也不能隐式重置用户要求的一次性标记。
    let mut changed = edit(store.get_host(&host.id).await.unwrap());
    changed.address = "other.example.com".into();
    store.save_host(changed).await.unwrap();
    let config = store.get_host_connection(&host.id).await.unwrap();
    assert!(!store.claim(&host.id, "changed", &config).await.unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn system_detection_toggle_and_manual_icon_take_priority() {
    use terminal_core::RemoteSystemStore;
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("system.sqlite3"))
        .await
        .unwrap();
    let host = store.save_host(draft()).await.unwrap();
    let config = store.get_host_connection(&host.id).await.unwrap();
    store
        .set_setting("app", 1, &serde_json::json!({"detectSshSystem": false}))
        .await
        .unwrap();
    assert!(!store.claim(&host.id, "off", &config).await.unwrap());
    assert!(
        !store
            .get_host(&host.id)
            .await
            .unwrap()
            .system_detection_attempted
    );
    store
        .set_setting("app", 1, &serde_json::json!({"detectSshSystem": true}))
        .await
        .unwrap();
    assert!(store.claim(&host.id, "on", &config).await.unwrap());
    let mut changed = edit(store.get_host(&host.id).await.unwrap());
    changed.icon = "debian".into();
    store.save_host(changed).await.unwrap();
    store
        .complete(&host.id, "on", &config, Some("ubuntu"))
        .await
        .unwrap();
    assert_eq!(store.get_host(&host.id).await.unwrap().icon, "debian");
    // 即使手动改回默认图标，也不能被迟到的旧检测结果覆盖。
    let mut changed = edit(store.get_host(&host.id).await.unwrap());
    changed.icon = "ssh".into();
    store.save_host(changed).await.unwrap();
    store
        .complete(&host.id, "on", &config, Some("ubuntu"))
        .await
        .unwrap();
    assert_eq!(store.get_host(&host.id).await.unwrap().icon, "ssh");
    assert!(!store.claim(&host.id, "again", &config).await.unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn system_detection_cannot_update_a_changed_target_or_deleted_host() {
    use terminal_core::RemoteSystemStore;
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("system.sqlite3"))
        .await
        .unwrap();
    let host = store.save_host(draft()).await.unwrap();
    let config = store.get_host_connection(&host.id).await.unwrap();
    let mut wrong = config.clone();
    wrong.address = "other.example.com".into();
    assert!(!store.claim(&host.id, "wrong", &wrong).await.unwrap());
    assert!(store.claim(&host.id, "original", &config).await.unwrap());
    let mut changed = edit(store.get_host(&host.id).await.unwrap());
    changed.address = wrong.address;
    store.save_host(changed).await.unwrap();
    store
        .complete(&host.id, "original", &config, Some("ubuntu"))
        .await
        .unwrap();
    assert_eq!(store.get_host(&host.id).await.unwrap().icon, "ssh");
    let mut changed = edit(store.get_host(&host.id).await.unwrap());
    changed.address = config.address.clone();
    store.save_host(changed).await.unwrap();
    store
        .complete(&host.id, "original", &config, Some("ubuntu"))
        .await
        .unwrap();
    let host = store.get_host(&host.id).await.unwrap();
    assert_eq!(host.icon, "ssh");
    store.delete_host(&host.id, host.revision).await.unwrap();
    store
        .complete(&host.id, "original", &config, Some("ubuntu"))
        .await
        .unwrap();
    assert!(store.get_host(&host.id).await.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn system_detection_success_persists_and_rejects_non_builtin_icons() {
    use terminal_core::RemoteSystemStore;
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("system.sqlite3"))
        .await
        .unwrap();
    let host = store.save_host(draft()).await.unwrap();
    let config = store.get_host_connection(&host.id).await.unwrap();
    assert!(store.claim(&host.id, "success", &config).await.unwrap());
    store
        .complete(&host.id, "success", &config, Some("ubuntu"))
        .await
        .unwrap();
    let host = store.get_host(&host.id).await.unwrap();
    assert_eq!(host.icon, "ubuntu");
    assert!(host.system_detection_attempted);
    let mut changed = edit(host);
    changed.icon = "<svg onload=alert(1)>".into();
    assert!(store.save_host(changed).await.is_err());
}

fn telnet_draft() -> HostDraft {
    let mut host = draft();
    host.kind = "telnet".into();
    host.name = "Telnet 设备".into();
    host.port = 23;
    host.username.clear();
    host.auth_plan.clear();
    host.tags = vec!["设备".into()];
    host
}

#[tokio::test(flavor = "current_thread")]
async fn telnet_hosts_round_trip_without_ssh_credentials_and_reject_type_changes() {
    let directory = TestDirectory::new();
    let path = directory.0.join("telnet.sqlite3");
    let store = SqliteStore::open(&path).await.unwrap();
    let saved = store.save_host(telnet_draft()).await.unwrap();
    assert_eq!(saved.kind, "telnet");
    assert_eq!(saved.port, 23);
    assert!(saved.auth_plan.is_empty());
    assert!(!saved.has_password);
    assert!(store.get_host_connection(&saved.id).await.is_err());
    assert_eq!(store.list_hosts("example.com").await.unwrap().len(), 1);
    let mut changed = edit(saved.clone());
    changed.address = "2001:db8::1".into();
    changed.port = 2323;
    changed.favorite = true;
    let updated = store.save_host(changed).await.unwrap();
    let copy = store.duplicate_host(&updated.id).await.unwrap();
    assert_eq!(copy.address, updated.address);
    assert_eq!(copy.port, 2323);
    assert_eq!(copy.tags, updated.tags);
    assert!(copy.favorite);
    let mut changed_kind = edit(updated.clone());
    changed_kind.kind = "ssh".into();
    changed_kind.username = "user".into();
    changed_kind.auth_plan = draft().auth_plan;
    assert!(store.save_host(changed_kind).await.is_err());
    let mut invalid = telnet_draft();
    invalid.auth_plan = draft().auth_plan;
    assert!(store.save_host(invalid).await.is_err());
    let mut invalid = telnet_draft();
    invalid.username = "user".into();
    assert!(store.save_host(invalid).await.is_err());
    let mut invalid = telnet_draft();
    invalid.serial_config = serial_draft().serial_config;
    assert!(store.save_host(invalid).await.is_err());
    assert!(
        store
            .save_host_with_password_ref(telnet_draft(), Some("forged"))
            .await
            .is_err()
    );
    assert!(store.list_all_credentials().await.unwrap().is_empty());
    store.pool().await.unwrap().close().await;
    let restored = SqliteStore::open(&path).await.unwrap();
    let host = restored.get_host(&updated.id).await.unwrap();
    assert_eq!(host.address, "2001:db8::1");
    assert_eq!(host.port, 2323);
    assert_eq!(host.revision, updated.revision);
    restored.pool().await.unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn schema_two_upgrade_preserves_profiles_children_and_history_references() {
    let directory = TestDirectory::new();
    let path = directory.0.join("upgrade.sqlite3");
    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .pragma("key", format!("\"x'{}'\"", hex::encode([42u8; 32])))
        .pragma("cipher_compatibility", "4")
        .foreign_keys(true)
        .disable_statement_logging();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let mut initial = sqlx::migrate!("./migrations");
    initial.migrations = std::borrow::Cow::Owned(initial.iter().take(1).cloned().collect());
    initial.run(&pool).await.unwrap();
    sqlx::query("PRAGMA user_version = 2")
        .execute(&pool)
        .await
        .unwrap();
    let old = SqliteStore::locked(&path);
    *old.pool.write().await = Some(pool);
    old.unlocked.store(true, Ordering::Release);
    let group = old
        .save_group(GroupDraft {
            id: None,
            revision: None,
            name: "设备".into(),
            parent_id: None,
            sort_order: 0,
        })
        .await
        .unwrap();
    let owner = uuid::Uuid::new_v4().to_string();
    let secret = old.begin_host_password(&owner, "主机密码").await.unwrap();
    old.set_credential_secret(&secret, "fixture-secret")
        .await
        .unwrap();
    let mut ssh = draft();
    ssh.group_id = Some(group.id.clone());
    ssh.tags = vec!["保留".into()];
    let ssh = old
        .save_host_with_password_ref(ssh, Some(&secret))
        .await
        .unwrap();
    let serial = old.save_host(serial_draft()).await.unwrap();
    // 旧 schema 的夹具直接写 SQL，产品入口不再读取或维护旧连接历史。
    sqlx::query("INSERT INTO session_history(id,profile_id,display_name_snapshot,started_at) VALUES('legacy',?,?,1)")
        .bind(&ssh.id).bind(&ssh.name).execute(&*old.pool().await.unwrap()).await.unwrap();
    old.set_setting("fixture", 1, &serde_json::json!({"preserved":true}))
        .await
        .unwrap();
    // 升级前类型约束确实不接受 Telnet。
    assert!(old.save_host(telnet_draft()).await.is_err());
    old.pool().await.unwrap().close().await;
    let store = SqliteStore::open(&path).await.unwrap();
    let restored = store.get_host(&ssh.id).await.unwrap();
    assert_eq!(restored.group_id, Some(group.id));
    assert_eq!(restored.tags, ssh.tags);
    assert_eq!(restored.revision, ssh.revision);
    assert!(restored.has_password);
    let config = store.get_host_connection(&ssh.id).await.unwrap();
    assert!(
        matches!(&config.auth_plan[0], AuthStep::Password { credential_id: Some(id) } if id == &secret)
    );
    assert_eq!(store.get_host(&serial.id).await.unwrap().kind, "serial");
    let profile_id: Option<String> =
        sqlx::query_scalar("SELECT profile_id FROM session_history WHERE id='legacy'")
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(profile_id, Some(ssh.id));
    assert_eq!(
        store.get_setting("fixture").await.unwrap().unwrap().value["preserved"],
        true
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&*store.pool().await.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
    for _ in 0..2 {
        let mut connection = store.pool().await.unwrap().acquire().await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
                .fetch_one(&mut *connection)
                .await
                .unwrap(),
            1
        );
    }
    assert_eq!(
        store.save_host(telnet_draft()).await.unwrap().kind,
        "telnet"
    );
    store.pool().await.unwrap().close().await;
    // 重开不重复迁移或删除关联。
    let reopened = SqliteStore::open(&path).await.unwrap();
    assert_eq!(reopened.list_hosts("").await.unwrap().len(), 3);
    reopened.pool().await.unwrap().close().await;
}

fn workspace_draft(name: &str, profile_id: &str) -> WorkspaceDraft {
    WorkspaceDraft {
        name: name.into(),
        expected_revision: None,
        workspace: SavedWorkspace {
            root: SavedLayout::Split {
                axis: "vertical".into(),
                ratio: 0.37,
                first: Box::new(SavedLayout::Connection {
                    profile_id: profile_id.into(),
                }),
                second: Box::new(SavedLayout::Connection {
                    profile_id: profile_id.into(),
                }),
            },
            focused_index: 1,
        },
    }
}
#[tokio::test(flavor = "current_thread")]
async fn names_are_unique_across_all_saved_profile_kinds() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("names.sqlite3"))
        .await
        .unwrap();
    let ssh = store.save_host(draft()).await.unwrap();
    let serial = store.save_host(serial_draft()).await.unwrap();
    let telnet = store.save_host(telnet_draft()).await.unwrap();
    let workspace = store
        .save_workspace(workspace_draft("工作区", &ssh.id))
        .await
        .unwrap();
    for saved in [&ssh, &serial, &telnet, &workspace] {
        for mut candidate in [draft(), serial_draft(), telnet_draft()] {
            candidate.name = format!(" {} ", saved.name);
            assert!(matches!(
                store.save_host(candidate).await,
                Err(StorageError::Validation(_))
            ));
        }
    }
    // 保留原名可以编辑，跨类型重名修改必须回滚。
    let saved = store.save_host(edit(ssh.clone())).await.unwrap();
    let mut collision = edit(saved.clone());
    collision.name = serial.name;
    assert!(store.save_host(collision).await.is_err());
    assert_eq!(
        store.get_host(&ssh.id).await.unwrap().revision,
        saved.revision
    );
    let (first, second) = tokio::join!(
        store.save_workspace(workspace_draft("并发工作区", &ssh.id)),
        store.save_workspace(workspace_draft("并发工作区", &ssh.id)),
    );
    assert_ne!(first.is_ok(), second.is_ok());
    store.pool().await.unwrap().close().await;
}
#[tokio::test(flavor = "current_thread")]
async fn workspace_replacement_preserves_identity_and_rejects_other_kinds_or_stale_writes() {
    let directory = TestDirectory::new();
    let store = SqliteStore::open(directory.0.join("workspaces.sqlite3"))
        .await
        .unwrap();
    let host = store.save_host(draft()).await.unwrap();
    let original = store
        .save_workspace(workspace_draft("开发工作区", &host.id))
        .await
        .unwrap();
    let group = group_fixture(&store, "工作区分组", None).await;
    let mut metadata = edit(original.clone());
    metadata.favorite = true;
    metadata.group_id = Some(group.id.clone());
    let current = store.save_host(metadata).await.unwrap();
    let mut replacement = workspace_draft(" 开发工作区 ", &host.id);
    replacement.expected_revision = Some(current.revision);
    replacement.workspace.root = SavedLayout::Connection {
        profile_id: host.id.clone(),
    };
    replacement.workspace.focused_index = 0;
    let saved = store.save_workspace(replacement.clone()).await.unwrap();
    assert_eq!(saved.id, original.id);
    assert_eq!(saved.group_id, Some(group.id));
    assert!(saved.favorite);
    assert_eq!(saved.revision, current.revision + 1);
    let reloaded = store.get_host(&saved.id).await.unwrap();
    assert_eq!(
        serde_json::to_value(reloaded.workspace).unwrap(),
        serde_json::to_value(saved.workspace.clone()).unwrap()
    );
    assert!(matches!(
        store.save_workspace(replacement).await,
        Err(StorageError::Conflict)
    ));
    assert!(
        store
            .save_workspace(workspace_draft(&host.name, &host.id))
            .await
            .is_err()
    );
    assert!(
        store
            .save_workspace(workspace_draft("嵌套工作区", &saved.id))
            .await
            .is_err()
    );
    assert!(store.get_host_connection(&saved.id).await.is_err());
    let mut forbidden = edit(saved.clone());
    forbidden.name = "不能重命名".into();
    assert!(store.save_host(forbidden).await.is_err());
    let mut manual = edit(saved.clone());
    manual.id = None;
    manual.name = "手动创建".into();
    assert!(store.save_host(manual).await.is_err());
    // 删除连接保留失效工作区，后续保存必须重新选取有效引用。
    store.delete_host(&host.id, host.revision).await.unwrap();
    assert!(store.get_host(&saved.id).await.unwrap().workspace.is_some());
    assert!(
        store
            .save_workspace(workspace_draft("失效引用", &host.id))
            .await
            .is_err()
    );
    store.pool().await.unwrap().close().await;
}
#[test]
fn saved_workspace_dto_rejects_secrets_and_session_fields() {
    let value = serde_json::json!({"name":"工作区","expectedRevision":null,"workspace":{"root":{"kind":"connection","profileId":"id","password":"secret"},"focusedIndex":0}});
    assert!(serde_json::from_value::<WorkspaceDraft>(value).is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn schema_four_upgrade_renames_duplicates_without_changing_references() {
    let directory = TestDirectory::new();
    let path = directory.0.join("workspace-upgrade.sqlite3");
    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .pragma("key", format!("\"x'{}'\"", hex::encode([42u8; 32])))
        .pragma("cipher_compatibility", "4")
        .foreign_keys(true)
        .disable_statement_logging();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let mut initial = sqlx::migrate!("./migrations");
    initial.migrations = std::borrow::Cow::Owned(initial.iter().take(3).cloned().collect());
    initial.run(&pool).await.unwrap();
    for (id, kind, name, created) in [
        ("old-ssh", "ssh", "重复", 1i64),
        ("old-serial", "serial", "重复", 2),
        ("collision", "telnet", "重复 (2)", 3),
    ] {
        sqlx::query("INSERT INTO connection_profiles(id,kind,name,favorite,created_at,updated_at) VALUES(?,?,?,0,?,?)")
            .bind(id).bind(kind).bind(name).bind(created).bind(created).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO ssh_profiles VALUES('old-ssh','example.com','example.com',22,'user')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO port_forwards(id,name,profile_id,rule_json,created_at,updated_at) VALUES('rule','转发','old-ssh','{}',1,1)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO session_history(id,profile_id,display_name_snapshot,started_at,ended_at) VALUES('history','old-ssh','重复',1,2)").execute(&pool).await.unwrap();
    pool.close().await;
    let store = SqliteStore::open(&path).await.unwrap();
    let names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM connection_profiles ORDER BY created_at")
            .fetch_all(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(names, vec!["重复", "重复 (5)", "重复 (2)"]);
    let profile: Option<String> =
        sqlx::query_scalar("SELECT profile_id FROM port_forwards WHERE id='rule'")
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(profile.as_deref(), Some("old-ssh"));
    let profile: Option<String> =
        sqlx::query_scalar("SELECT profile_id FROM session_history WHERE id='history'")
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(profile.as_deref(), Some("old-ssh"));
    assert!(store.get_host_connection("old-ssh").await.is_ok());
    let duplicate = sqlx::query("INSERT INTO connection_profiles(id,kind,name,favorite,created_at,updated_at) VALUES('new','workspace',' 重复 ',0,1,1)").execute(&*store.pool().await.unwrap()).await;
    assert!(duplicate.is_err());
    store.pool().await.unwrap().close().await;
}

#[test]
fn applied_workspace_migration_keeps_its_original_checksum() {
    use sha2::Digest;
    // 固定实际运行过的初版迁移摘要，后续调整只能追加迁移文件。
    let checksum = sha2::Sha384::digest(include_bytes!("../migrations/0004_workspaces.sql"));
    assert_eq!(
        hex::encode(checksum),
        "cee4f9328cb293cb70c2c2f2009d0dadcb0a80b73e843a2fad34e421f3801517fb71f65f81061409bf2871ed3ded0f29"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn applied_workspace_migration_upgrades_without_rewriting_history_or_profiles() {
    let directory = TestDirectory::new();
    let path = directory.0.join("applied-workspaces.sqlite3");
    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .pragma("key", format!("\"x'{}'\"", hex::encode([42u8; 32])))
        .pragma("cipher_compatibility", "4")
        .foreign_keys(true)
        .disable_statement_logging();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let mut initial = sqlx::migrate!("./migrations");
    initial.migrations = std::borrow::Cow::Owned(initial.iter().take(4).cloned().collect());
    initial.run(&pool).await.unwrap();
    let old = SqliteStore::locked(&path);
    *old.pool.write().await = Some(pool);
    old.unlocked.store(true, Ordering::Release);
    let host = old.save_host(draft()).await.unwrap();
    let workspace = old
        .save_workspace(workspace_draft("升级保留工作区", &host.id))
        .await
        .unwrap();
    let before: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT version,checksum FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&*old.pool().await.unwrap())
            .await
            .unwrap();
    old.pool().await.unwrap().close().await;
    let store = SqliteStore::open(&path).await.unwrap();
    let after: Vec<(i64, Vec<u8>)> = sqlx::query_as(
        "SELECT version,checksum FROM _sqlx_migrations WHERE version<=4 ORDER BY version",
    )
    .fetch_all(&*store.pool().await.unwrap())
    .await
    .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        serde_json::to_value(store.get_host(&host.id).await.unwrap()).unwrap(),
        serde_json::to_value(host).unwrap()
    );
    assert_eq!(
        serde_json::to_value(store.get_host(&workspace.id).await.unwrap()).unwrap(),
        serde_json::to_value(workspace).unwrap()
    );
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&*store.pool().await.unwrap())
        .await
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    let index: String =
        sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE name='profiles_unique_name'")
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert!(index.contains("trim(name)"));
    store.pool().await.unwrap().close().await;
}

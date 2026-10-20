//! 同步冲突、认证一致性与事务保护的回归源码；遵循仓库规则，本轮不执行。
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use sha2::{Digest, Sha256};

fn text(value: &str) -> Cell {
    Cell::Text(Zeroizing::new(value.to_owned()))
}
fn number(value: i64) -> Cell {
    Cell::Integer(value)
}
fn empty() -> Data {
    TABLES.iter().map(|_| Vec::new()).collect()
}

fn add_host(data: &mut Data, id: &str, name: &str, credential: Option<(&str, &str)>) {
    data[PROFILES].push(vec![
        text(id),
        text("ssh"),
        text(name),
        Cell::Null,
        number(0),
        number(1),
        text(r#"{"icon":"ssh"}"#),
        number(1),
        number(1),
        number(1),
    ]);
    data[SSH].push(vec![
        text(id),
        text("example.com"),
        text("example.com"),
        number(22),
        text("user"),
    ]);
    data[AUTH].push(vec![
        text(&format!("auth-{id}")),
        text(id),
        number(0),
        text("password"),
        credential.map(|(id, _)| text(id)).unwrap_or(Cell::Null),
    ]);
    if let Some((credential_id, secret)) = credential {
        data[CREDENTIALS].push(vec![
            text(credential_id),
            text("password"),
            text("主机密码"),
            text(id),
            text(secret),
            text("ready"),
            number(1),
            number(1),
        ]);
    }
}
fn remove_host(data: &mut Data, id: &str) {
    for (table, column) in [
        (PROFILES, 0),
        (SSH, 0),
        (AUTH, 1),
        (CREDENTIALS, 3),
        (LINKS, 0),
    ] {
        data[table].retain(|row| row[column].optional_text().unwrap() != Some(id));
    }
}
fn add_group(data: &mut Data, id: &str, parent: Option<&str>) {
    data[GROUPS].push(vec![
        text(id),
        parent.map(text).unwrap_or(Cell::Null),
        text(id),
        number(0),
        number(1),
        number(1),
        number(1),
    ]);
}
fn add_snippet(data: &mut Data, id: &str, content: &str) {
    data[SNIPPETS].push(vec![
        text(id),
        text(id),
        text(content),
        number(1),
        number(1),
        number(1),
    ]);
}
fn merged(base: &Data, local: &Data, remote: &Data) -> Data {
    for data in [base, local, remote] {
        backup::validate(data).unwrap();
    }
    merge(base, local, remote, &ProtectedReferences::default()).unwrap()
}

#[test]
fn single_side_child_and_owned_secret_edits_are_not_lost_when_parent_is_unchanged() {
    let mut base = empty();
    add_host(
        &mut base,
        "host-a",
        "主机",
        Some(("credential-a", "old-secret")),
    );
    let mut remote = base.clone();
    remote[SSH][0][4] = text("new-user");
    remote[CREDENTIALS][0][4] = text("new-secret");
    let result = merged(&base, &base, &remote);
    assert!(result[SSH][0][4] == text("new-user"));
    assert!(result[CREDENTIALS][0][4] == text("new-secret"));
    assert!(result[PROFILES][0][7].integer().unwrap() > 1);
}

#[test]
fn concurrent_authentication_changes_select_one_complete_bundle() {
    let mut base = empty();
    add_host(
        &mut base,
        "host-a",
        "主机",
        Some(("credential-a", "old-secret")),
    );
    let mut local = base.clone();
    local[CREDENTIALS][0][0] = text("credential-local");
    local[CREDENTIALS][0][4] = text("local-secret");
    local[AUTH][0][4] = text("credential-local");
    let mut remote = base.clone();
    remote[SSH][0][4] = text("remote-user");
    remote[CREDENTIALS][0][4] = text("remote-secret");
    let result = merged(&base, &local, &remote);
    assert_eq!(result[CREDENTIALS].len(), 1);
    assert!(result[CREDENTIALS][0][0] == text("credential-local"));
    assert!(result[CREDENTIALS][0][4] == text("local-secret"));
    assert!(result[AUTH][0][4] == text("credential-local"));
    assert!(result[SSH][0][4] == text("user"));
}

#[test]
fn deletion_propagates_but_concurrent_connection_change_preserves_the_complete_host() {
    let mut base = empty();
    add_host(
        &mut base,
        "host-a",
        "主机",
        Some(("credential-a", "secret")),
    );
    let deleted = empty();
    assert!(merged(&base, &base, &deleted)[PROFILES].is_empty());
    let mut changed = base.clone();
    changed[SSH][0][4] = text("modified-user");
    for (local, remote) in [(&deleted, &changed), (&changed, &deleted)] {
        let result = merged(&base, local, remote);
        assert_eq!(result[PROFILES].len(), 1);
        assert_eq!(result[SSH].len(), 1);
        assert_eq!(result[AUTH].len(), 1);
        assert_eq!(result[CREDENTIALS].len(), 1);
    }
}

#[test]
fn a_new_tag_reference_restores_deleted_hosts_with_their_authentication() {
    let mut base = empty();
    add_host(
        &mut base,
        "host-a",
        "主机",
        Some(("credential-a", "secret")),
    );
    let local = empty();
    let mut remote = base.clone();
    remote[TAGS].push(vec![text("tag-a"), text("新标签")]);
    remote[LINKS].push(vec![text("host-a"), text("tag-a")]);
    let result = merged(&base, &local, &remote);
    assert_eq!(result[PROFILES].len(), 1);
    assert_eq!(result[SSH].len(), 1);
    assert_eq!(result[AUTH].len(), 1);
    assert_eq!(result[CREDENTIALS].len(), 1);
    assert_eq!(result[LINKS].len(), 1);
}

#[test]
fn dependency_closure_restores_more_than_ten_ancestors() {
    let mut base = empty();
    let mut parent = None;
    for index in 0..24 {
        let id = format!("group-{index}");
        add_group(&mut base, &id, parent.as_deref());
        parent = Some(id);
    }
    let local = empty();
    let mut remote = base.clone();
    add_host(&mut remote, "host-a", "远端新主机", None);
    remote[PROFILES][0][3] = text(parent.as_deref().unwrap());
    let result = merged(&base, &local, &remote);
    assert_eq!(result[GROUPS].len(), 24);
    assert_eq!(result[PROFILES].len(), 1);
}

#[test]
fn independently_valid_group_edits_that_form_a_cycle_are_rejected() {
    let mut base = empty();
    add_group(&mut base, "group-a", None);
    add_group(&mut base, "group-b", None);
    let mut local = base.clone();
    local[GROUPS][0][1] = text("group-b");
    let mut remote = base.clone();
    remote[GROUPS][1][1] = text("group-a");
    backup::validate(&local).unwrap();
    backup::validate(&remote).unwrap();
    assert!(merge(&base, &local, &remote, &ProtectedReferences::default()).is_err());
}

#[test]
fn duplicate_names_have_stable_ids_and_do_not_steal_preexisting_suffix_names() {
    let mut local = empty();
    add_host(&mut local, "aaaaaaaa-1", "同名主机", None);
    add_host(&mut local, "zzzzzzzz-3", "同名主机 (bbbbbbbb-1)", None);
    let mut remote = empty();
    add_host(&mut remote, "bbbbbbbb-2", "同名主机", None);
    let result = merged(&empty(), &local, &remote);
    assert_eq!(result[PROFILES].len(), 3);
    assert!(rows(PROFILES, &result).unwrap()["bbbbbbbb-2"][2] == text("同名主机 (bbbbbbbb-2)"));
    let again = merged(&result, &result, &result);
    assert!(equivalent(&result, &again).unwrap());
}

fn trust(id: &str, value: u8) -> Row {
    let mut public_key = Vec::new();
    public_key.extend_from_slice(&11u32.to_be_bytes());
    public_key.extend_from_slice(b"ssh-ed25519");
    public_key.extend_from_slice(&32u32.to_be_bytes());
    public_key.extend_from_slice(&[value; 32]);
    let fingerprint = format!(
        "SHA256:{}",
        STANDARD_NO_PAD.encode(Sha256::digest(&public_key))
    );
    vec![
        text(id),
        text("example.com"),
        number(22),
        text("ssh-ed25519"),
        Cell::Blob(Zeroizing::new(public_key)),
        text(&fingerprint),
        number(1),
        Cell::Null,
    ]
}

#[test]
fn a_different_key_for_the_same_endpoint_is_rejected_even_when_record_ids_match() {
    let mut local = empty();
    local[TRUST].push(trust("same-id", 1));
    let mut remote = empty();
    remote[TRUST].push(trust("same-id", 2));
    for data in [&local, &remote] {
        backup::validate(data).unwrap();
    }
    assert!(merge(&empty(), &local, &remote, &ProtectedReferences::default()).is_err());
}

#[test]
fn equivalent_databases_ignore_physical_row_order_but_compare_secret_content() {
    let mut left = empty();
    add_host(
        &mut left,
        "host-a",
        "主机",
        Some(("credential-a", "secret")),
    );
    add_snippet(&mut left, "snippet-a", "echo a");
    add_snippet(&mut left, "snippet-b", "echo b");
    let mut right = left.clone();
    right[SNIPPETS].reverse();
    let first = backup_sqlite::encode(&left).unwrap();
    let second = backup_sqlite::encode(&right).unwrap();
    assert!(equivalent_official_databases(&first, &second).unwrap());
    right[CREDENTIALS][0][4] = text("different-secret");
    let changed = backup_sqlite::encode(&right).unwrap();
    assert!(!equivalent_official_databases(&first, &changed).unwrap());
    assert!(equivalent_official_databases(&first, b"not-a-database").is_err());
}

struct TestDirectory(std::path::PathBuf);
impl TestDirectory {
    async fn store() -> (Self, SqliteStore) {
        let path =
            std::env::temp_dir().join(format!("termeta-official-sync-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        let store = SqliteStore::open(path.join("vault.sqlite3")).await.unwrap();
        (Self(path), store)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
async fn seed(store: &SqliteStore, data: &Data) {
    backup::validate(data).unwrap();
    let pool = store.pool().await.unwrap();
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    sqlx::query("PRAGMA defer_foreign_keys=ON")
        .execute(&mut *tx)
        .await
        .unwrap();
    for (index, table) in TABLES.iter().enumerate() {
        for row in &data[index] {
            backup::insert(&mut tx, table, row).await.unwrap();
        }
    }
    tx.commit().await.unwrap();
}
async fn history(store: &SqliteStore, id: &str, profile: &str) {
    sqlx::query("INSERT INTO session_history(id,profile_id,kind,display_name_snapshot,started_at) VALUES(?,?,'ssh','历史',1)").bind(id).bind(profile).execute(&*store.pool().await.unwrap()).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn no_op_polls_do_not_write_business_tables_or_clear_detection_tokens() {
    let (_directory, store) = TestDirectory::store().await;
    let mut data = empty();
    add_host(&mut data, "host-a", "主机", None);
    data[PROFILES][0][6] = text(
        r#"{"icon":"ssh","systemDetectionAttempted":true,"systemDetectionToken":"local-detection-token"}"#,
    );
    seed(&store, &data).await;
    let snapshot = store.export_backup_database().await.unwrap();
    sqlx::query("CREATE TABLE sync_probe(writes INTEGER NOT NULL)")
        .execute(&*store.pool().await.unwrap())
        .await
        .unwrap();
    sqlx::query("INSERT INTO sync_probe VALUES(0)")
        .execute(&*store.pool().await.unwrap())
        .await
        .unwrap();
    for table in TABLES {
        for operation in ["INSERT", "UPDATE", "DELETE"] {
            sqlx::query(&format!("CREATE TRIGGER probe_{}_{} AFTER {operation} ON {} BEGIN UPDATE sync_probe SET writes=writes+1; END",table.name,operation,table.name)).execute(&*store.pool().await.unwrap()).await.unwrap();
        }
    }
    let result = store
        .merge_official_database(snapshot.clone(), Some(snapshot.clone()), None)
        .await
        .unwrap();
    assert!(equivalent_official_databases(&snapshot, &result).unwrap());
    let writes: i64 = sqlx::query_scalar("SELECT writes FROM sync_probe")
        .fetch_one(&*store.pool().await.unwrap())
        .await
        .unwrap();
    assert_eq!(writes, 0);
    let token:String=sqlx::query_scalar("SELECT json_extract(config_json,'$.systemDetectionToken') FROM connection_profiles WHERE id='host-a'").fetch_one(&*store.pool().await.unwrap()).await.unwrap();
    assert_eq!(token, "local-detection-token");
    let mut remote = decode(result.clone()).unwrap();
    add_snippet(&mut remote, "remote-snippet", "echo remote");
    let latest = store
        .merge_official_database(backup_sqlite::encode(&remote).unwrap(), Some(result), None)
        .await
        .unwrap();
    let writes: i64 = sqlx::query_scalar("SELECT writes FROM sync_probe")
        .fetch_one(&*store.pool().await.unwrap())
        .await
        .unwrap();
    assert_eq!(writes, 1);
    remote = decode(latest.clone()).unwrap();
    remote[PROFILES][0][2] = text("仅修改名称");
    store
        .merge_official_database(backup_sqlite::encode(&remote).unwrap(), Some(latest), None)
        .await
        .unwrap();
    let token: String = sqlx::query_scalar("SELECT json_extract(config_json,'$.systemDetectionToken') FROM connection_profiles WHERE id='host-a'").fetch_one(&*store.pool().await.unwrap()).await.unwrap();
    assert_eq!(token, "local-detection-token");
    store.pool().await.unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn changed_sync_preserves_local_profiles_unrelated_history_settings_and_pending_credentials()
{
    let (_directory, store) = TestDirectory::store().await;
    let mut base = empty();
    add_group(&mut base, "local-group", None);
    add_host(&mut base, "host-delete", "删除主机", None);
    base[PROFILES][0][3] = text("local-group");
    add_host(&mut base, "host-keep", "保留主机", None);
    base[TAGS].push(vec![text("local-tag"), text("本地标签")]);
    base[LINKS].push(vec![text("host-delete"), text("local-tag")]);
    seed(&store, &base).await;
    history(&store, "history-keep", "host-keep").await;
    history(&store, "history-delete", "host-delete").await;
    sqlx::query("INSERT INTO connection_profiles(id,kind,name,group_id,favorite,config_version,config_json,revision,created_at,updated_at) VALUES('local-only','local','保留主机','local-group',0,1,'{}',9,1,1)").execute(&*store.pool().await.unwrap()).await.unwrap();
    sqlx::query("INSERT INTO profile_tags VALUES('local-only','local-tag')")
        .execute(&*store.pool().await.unwrap())
        .await
        .unwrap();
    sqlx::query("INSERT INTO credentials(id,kind,label,status,created_at,updated_at) VALUES('pending-credential','password','准备中','pending',1,1)").execute(&*store.pool().await.unwrap()).await.unwrap();
    store
        .set_setting("app", 1, &serde_json::json!({"theme":"local-setting"}))
        .await
        .unwrap();
    let mut remote = base.clone();
    remove_host(&mut remote, "host-delete");
    remote[GROUPS].clear();
    remote[TAGS].clear();
    add_snippet(&mut remote, "remote-snippet", "echo remote");
    store
        .merge_official_database(
            backup_sqlite::encode(&remote).unwrap(),
            Some(backup_sqlite::encode(&base).unwrap()),
            None,
        )
        .await
        .unwrap();
    let local: (String, String, i64) = sqlx::query_as(
        "SELECT name,group_id,revision FROM connection_profiles WHERE id='local-only'",
    )
    .fetch_one(&*store.pool().await.unwrap())
    .await
    .unwrap();
    assert_eq!(local, ("保留主机".to_owned(), "local-group".to_owned(), 9));
    let local_link: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM profile_tags WHERE profile_id='local-only' AND tag_id='local-tag'",
    )
    .fetch_one(&*store.pool().await.unwrap())
    .await
    .unwrap();
    assert_eq!(local_link, 1);
    let kept: String =
        sqlx::query_scalar("SELECT profile_id FROM session_history WHERE id='history-keep'")
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(kept, "host-keep");
    let removed: Option<String> =
        sqlx::query_scalar("SELECT profile_id FROM session_history WHERE id='history-delete'")
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert!(removed.is_none());
    let status: String =
        sqlx::query_scalar("SELECT status FROM credentials WHERE id='pending-credential'")
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(status, "pending");
    assert_eq!(
        store.get_setting("app").await.unwrap().unwrap().value["theme"],
        "local-setting"
    );
    assert_eq!(store.list_snippets().await.unwrap().len(), 1);
    store.pool().await.unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn names_can_swap_and_failed_later_writes_roll_back_the_entire_merge() {
    let (_directory, store) = TestDirectory::store().await;
    let mut base = empty();
    add_host(&mut base, "host-a", "名称甲", None);
    add_host(&mut base, "host-b", "名称乙", None);
    seed(&store, &base).await;
    history(&store, "history-a", "host-a").await;
    let mut remote = base.clone();
    remote[PROFILES][0][2] = text("名称乙");
    remote[PROFILES][1][2] = text("名称甲");
    let after = store
        .merge_official_database(
            backup_sqlite::encode(&remote).unwrap(),
            Some(backup_sqlite::encode(&base).unwrap()),
            None,
        )
        .await
        .unwrap();
    assert_eq!(store.get_host("host-a").await.unwrap().name, "名称乙");
    assert_eq!(store.get_host("host-b").await.unwrap().name, "名称甲");
    let mut failing = decode(after.clone()).unwrap();
    failing[PROFILES][0][2] = text("尚未提交名称");
    add_snippet(&mut failing, "fail-snippet", "echo denied");
    sqlx::query("CREATE TRIGGER fail_sync_insert BEFORE INSERT ON snippets BEGIN SELECT RAISE(ABORT,'injected failure'); END").execute(&*store.pool().await.unwrap()).await.unwrap();
    assert!(
        store
            .merge_official_database(
                backup_sqlite::encode(&failing).unwrap(),
                Some(after.clone()),
                None,
            )
            .await
            .is_err()
    );
    let untouched = store.export_backup_database().await.unwrap();
    assert!(equivalent_official_databases(&after, &untouched).unwrap());
    let profile: String =
        sqlx::query_scalar("SELECT profile_id FROM session_history WHERE id='history-a'")
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(profile, "host-a");
    store.pool().await.unwrap().close().await;
}

#[test]
fn shared_credentials_are_independent_of_the_host_bundle_and_remain_consistent() {
    let mut base = empty();
    add_host(
        &mut base,
        "host-a",
        "主机",
        Some(("shared-credential", "old-secret")),
    );
    base[CREDENTIALS][0][3] = Cell::Null;
    let mut local = base.clone();
    local[PROFILES][0][2] = text("本地改名");
    let mut remote = base.clone();
    remote[CREDENTIALS][0][4] = text("remote-new-secret");
    let result = merged(&base, &local, &remote);
    assert!(result[PROFILES][0][2] == text("本地改名"));
    assert!(result[AUTH][0][4] == text("shared-credential"));
    assert!(result[CREDENTIALS][0][4] == text("remote-new-secret"));
}

#[test]
fn propagated_revision_metadata_converges_without_incrementing_on_every_device() {
    let mut base = empty();
    add_host(&mut base, "host-a", "原名称", None);
    let mut remote = base.clone();
    remote[PROFILES][0][2] = text("新名称");
    let first = merged(&base, &base, &remote);
    assert!(first[PROFILES][0][7].integer().unwrap() > remote[PROFILES][0][7].integer().unwrap());
    let accepted = merged(&remote, &remote, &first);
    assert!(equivalent(&first, &accepted).unwrap());
    let steady = merged(&first, &first, &accepted);
    assert!(equivalent(&first, &steady).unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn baselines_are_isolated_by_origin_and_account_and_stay_out_of_snapshots() {
    let (_directory, store) = TestDirectory::store().await;
    let first = backup_sqlite::encode(&empty()).unwrap();
    let mut data = empty();
    add_snippet(&mut data, "baseline-only-snippet", "secret-baseline-only");
    let second = backup_sqlite::encode(&data).unwrap();
    store
        .save_official_baseline("https://termeta.io", "account-a", 1, &first)
        .await
        .unwrap();
    store
        .save_official_baseline("https://termeta.io", "account-b", 2, &second)
        .await
        .unwrap();
    assert!(
        store
            .official_baseline("https://other.example", "account-a")
            .await
            .unwrap()
            .is_none()
    );
    let restored = store
        .official_baseline("https://termeta.io", "account-a")
        .await
        .unwrap()
        .unwrap();
    assert!(equivalent_official_databases(&first, &restored).unwrap());
    let exported = store.export_backup_database().await.unwrap();
    assert!(equivalent_official_databases(&first, &exported).unwrap());
    store.pool().await.unwrap().close().await;
}

fn observation(revision: i64) -> OfficialSyncObservation<'static> {
    OfficialSyncObservation {
        origin: "https://termeta.io",
        account_id: "account-a",
        revision,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn observed_baseline_prevents_cas_retry_or_restart_from_replaying_downloaded_changes() {
    let (_directory, store) = TestDirectory::store().await;
    let mut base = empty();
    add_host(
        &mut base,
        "host-a",
        "云端 A",
        Some(("credential-a", "secret")),
    );
    seed(&store, &base).await;
    let mut remote_b = base.clone();
    remote_b[PROFILES][0][2] = text("云端 B");
    let source_b = backup_sqlite::encode(&remote_b).unwrap();
    let imported_b = store
        .merge_official_database(
            source_b.clone(),
            Some(backup_sqlite::encode(&base).unwrap()),
            Some(observation(2)),
        )
        .await
        .unwrap();
    // 模拟 PUT 返回409、响应丢失或上传前退出：不保存任何“上传成功”基线。
    let observed = store
        .official_baseline("https://termeta.io", "account-a")
        .await
        .unwrap()
        .unwrap();
    assert!(equivalent_official_databases(&source_b, &observed).unwrap());
    assert!(
        decode(imported_b).unwrap()[PROFILES][0][7]
            .integer()
            .unwrap()
            > remote_b[PROFILES][0][7].integer().unwrap()
    );
    let path = store.path().to_owned();
    store.pool().await.unwrap().close().await;
    let reopened = SqliteStore::open(path).await.unwrap();
    reopened
        .save_snippet(crate::SnippetDraft {
            id: None,
            revision: None,
            name: "真实本地新增".into(),
            content: "echo local-change".into(),
        })
        .await
        .unwrap();
    let mut remote_c = remote_b.clone();
    remote_c[PROFILES][0][2] = text("云端 C");
    let baseline = reopened
        .official_baseline("https://termeta.io", "account-a")
        .await
        .unwrap();
    reopened
        .merge_official_database(
            backup_sqlite::encode(&remote_c).unwrap(),
            baseline,
            Some(observation(3)),
        )
        .await
        .unwrap();
    assert_eq!(reopened.get_host("host-a").await.unwrap().name, "云端 C");
    assert_eq!(
        reopened.list_snippets().await.unwrap()[0].content,
        "echo local-change"
    );
    // 远端后续删除也不能被本机导入时提升的 revision 错当成“删除与修改并发”。
    let baseline = reopened
        .official_baseline("https://termeta.io", "account-a")
        .await
        .unwrap();
    reopened
        .merge_official_database(
            backup_sqlite::encode(&empty()).unwrap(),
            baseline,
            Some(observation(4)),
        )
        .await
        .unwrap();
    assert!(reopened.get_host("host-a").await.is_err());
    assert_eq!(reopened.list_snippets().await.unwrap().len(), 1);
    let snapshot = decode(reopened.export_backup_database().await.unwrap()).unwrap();
    assert!(snapshot[CREDENTIALS].is_empty());
    reopened.pool().await.unwrap().close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn merged_business_changes_and_observed_baseline_commit_or_roll_back_together() {
    let (_directory, store) = TestDirectory::store().await;
    let mut base = empty();
    add_host(&mut base, "host-a", "原始名称", None);
    seed(&store, &base).await;
    let original = backup_sqlite::encode(&base).unwrap();
    store
        .save_official_baseline("https://termeta.io", "account-a", 1, &original)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER reject_baseline_update BEFORE UPDATE ON official_sync_state BEGIN SELECT RAISE(ABORT,'injected baseline failure'); END")
        .execute(&*store.pool().await.unwrap()).await.unwrap();
    // 相同头版本重复观测不会重写大块基线，也不会触发拒绝 UPDATE 的探针。
    store
        .merge_official_database(
            original.clone(),
            Some(original.clone()),
            Some(observation(1)),
        )
        .await
        .unwrap();
    let mut next = base.clone();
    next[PROFILES][0][2] = text("尚未提交的新名称");
    assert!(
        store
            .merge_official_database(
                backup_sqlite::encode(&next).unwrap(),
                Some(original.clone()),
                Some(observation(2)),
            )
            .await
            .is_err()
    );
    let after = store.export_backup_database().await.unwrap();
    assert!(equivalent(&decode(original.clone()).unwrap(), &decode(after).unwrap()).unwrap());
    let baseline = store
        .official_baseline("https://termeta.io", "account-a")
        .await
        .unwrap()
        .unwrap();
    assert!(equivalent(&decode(original).unwrap(), &decode(baseline).unwrap()).unwrap());
    let revision: i64 =
        sqlx::query_scalar("SELECT revision FROM official_sync_state WHERE account_id='account-a'")
            .fetch_one(&*store.pool().await.unwrap())
            .await
            .unwrap();
    assert_eq!(revision, 1);
    store.pool().await.unwrap().close().await;
}

#[test]
fn business_equivalence_ignores_local_revision_counters_but_keeps_actual_metadata_and_secrets() {
    let mut base = empty();
    add_group(&mut base, "group-a", None);
    add_host(
        &mut base,
        "host-a",
        "主机",
        Some(("credential-a", "secret")),
    );
    add_snippet(&mut base, "snippet-a", "echo value");
    let original = backup_sqlite::encode(&base).unwrap();
    let mut local = base.clone();
    for (table, column) in [(GROUPS, 4), (PROFILES, 7), (SNIPPETS, 3)] {
        local[table][0][column] = number(10);
    }
    let different_counters = backup_sqlite::encode(&local).unwrap();
    assert!(equivalent_official_databases(&original, &different_counters).unwrap());
    local[PROFILES][0][9] = number(99);
    assert!(
        !equivalent_official_databases(&original, &backup_sqlite::encode(&local).unwrap()).unwrap()
    );
    local[PROFILES][0][9] = number(1);
    local[CREDENTIALS][0][4] = text("changed-secret");
    assert!(
        !equivalent_official_databases(&original, &backup_sqlite::encode(&local).unwrap()).unwrap()
    );
}

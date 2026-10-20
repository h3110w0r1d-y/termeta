//! 官方同步按稳定 ID 三方合并，SSH 配置与专属凭据作为一个不可拆分的记录。
use crate::{
    Result, SqliteStore, StorageError,
    backup::{self, Cell, Data, TABLES},
    backup_sqlite,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use zeroize::Zeroizing;

const GROUPS: usize = 0;
const PROFILES: usize = 1;
const CREDENTIALS: usize = 2;
const SSH: usize = 3;
const AUTH: usize = 4;
const TAGS: usize = 5;
const LINKS: usize = 6;
const FORWARDS: usize = 7;
const TRUST: usize = 8;
const SNIPPETS: usize = 9;
type Row = Vec<Cell>;
type RowMap<'a> = BTreeMap<String, &'a Row>;

fn invalid() -> StorageError {
    StorageError::Validation("同步数据关联冲突，已保留本地数据")
}

fn key(index: usize, row: &[Cell]) -> Result<String> {
    Ok(if index == LINKS {
        format!("{}\0{}", row[0].text()?, row[1].text()?)
    } else {
        row[0].text()?.to_owned()
    })
}

fn rows(index: usize, data: &Data) -> Result<RowMap<'_>> {
    data[index]
        .iter()
        .map(|row| Ok((key(index, row)?, row)))
        .collect()
}

fn business_row_equal(index: usize, left: &[Cell], right: &[Cell]) -> bool {
    let revision = match index {
        GROUPS | FORWARDS => Some(4),
        PROFILES => Some(7),
        SNIPPETS => Some(3),
        _ => None,
    };
    // revision 是本机编辑草稿的乐观锁，不代表跨设备的业务修改；导入时会本地提升。
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .enumerate()
            .all(|(column, (left, right))| Some(column) == revision || left == right)
}

fn select<'a, T>(
    before: Option<&'a T>,
    here: Option<&'a T>,
    there: Option<&'a T>,
    equal: impl Fn(&T, &T) -> bool,
) -> Option<&'a T> {
    let same = |left: Option<&T>, right: Option<&T>| match (left, right) {
        (Some(left), Some(right)) => equal(left, right),
        (None, None) => true,
        _ => false,
    };
    // 单边变化直接采用；并发双改本地胜出，删除与修改并发保留修改。
    if same(here, before) {
        there
    } else if same(there, before) || same(here, there) {
        here
    } else {
        here.or(there)
    }
}

#[derive(PartialEq, Eq)]
struct ProfileBundle<'a> {
    profile: &'a Row,
    ssh: Option<&'a Row>,
    auth: BTreeMap<&'a str, &'a Row>,
    owned_credentials: BTreeMap<&'a str, &'a Row>,
}

fn bundles(data: &Data) -> Result<BTreeMap<String, ProfileBundle<'_>>> {
    let mut result = BTreeMap::new();
    for row in &data[PROFILES] {
        result.insert(
            row[0].text()?.to_owned(),
            ProfileBundle {
                profile: row,
                ssh: None,
                auth: BTreeMap::new(),
                owned_credentials: BTreeMap::new(),
            },
        );
    }
    for row in &data[SSH] {
        result.get_mut(row[0].text()?).ok_or_else(invalid)?.ssh = Some(row);
    }
    for row in &data[AUTH] {
        result
            .get_mut(row[1].text()?)
            .ok_or_else(invalid)?
            .auth
            .insert(row[0].text()?, row);
    }
    for row in &data[CREDENTIALS] {
        if let Some(owner) = row[3].optional_text()? {
            result
                .get_mut(owner)
                .ok_or_else(invalid)?
                .owned_credentials
                .insert(row[0].text()?, row);
        }
    }
    Ok(result)
}

fn append_bundle(result: &mut Data, bundle: &ProfileBundle<'_>) {
    result[PROFILES].push(bundle.profile.clone());
    if let Some(ssh) = bundle.ssh {
        result[SSH].push(ssh.clone());
    }
    result[AUTH].extend(bundle.auth.values().map(|row| (*row).clone()));
    result[CREDENTIALS].extend(bundle.owned_credentials.values().map(|row| (*row).clone()));
}

#[derive(Default)]
struct ProtectedReferences {
    // 旧版本的本机专属配置不进入快照，但它们的分组与标签引用不能被级联修改。
    groups: HashSet<String>,
    tags: HashSet<String>,
}

fn restore_references(
    result: &mut Data,
    local: &Data,
    remote: &Data,
    protected: &ProtectedReferences,
) -> Result<()> {
    let local_rows = (0..TABLES.len())
        .map(|index| rows(index, local))
        .collect::<Result<Vec<_>>>()?;
    let remote_rows = (0..TABLES.len())
        .map(|index| rows(index, remote))
        .collect::<Result<Vec<_>>>()?;
    let local_bundles = bundles(local)?;
    let remote_bundles = bundles(remote)?;
    let mut present = (0..TABLES.len())
        .map(|index| Ok(rows(index, result)?.into_keys().collect::<HashSet<_>>()))
        .collect::<Result<Vec<_>>>()?;
    loop {
        let mut needed = protected
            .groups
            .iter()
            .map(|id| (GROUPS, id.clone()))
            .chain(protected.tags.iter().map(|id| (TAGS, id.clone())))
            .collect::<Vec<_>>();
        for (table, column, target) in [
            (GROUPS, 1, GROUPS),
            (PROFILES, 3, GROUPS),
            (CREDENTIALS, 3, PROFILES),
            (AUTH, 4, CREDENTIALS),
            (LINKS, 0, PROFILES),
            (LINKS, 1, TAGS),
            (FORWARDS, 2, PROFILES),
        ] {
            for row in &result[table] {
                if let Some(id) = row[column].optional_text()? {
                    needed.push((target, id.to_owned()));
                }
            }
        }
        let mut changed = false;
        for (table, id) in needed {
            if present[table].contains(&id) {
                continue;
            }
            if table == PROFILES {
                let bundle = local_bundles
                    .get(&id)
                    .or_else(|| remote_bundles.get(&id))
                    .ok_or_else(invalid)?;
                append_bundle(result, bundle);
                present[PROFILES].insert(id);
                if let Some(row) = bundle.ssh {
                    present[SSH].insert(key(SSH, row)?);
                }
                for row in bundle.auth.values() {
                    present[AUTH].insert(key(AUTH, row)?);
                }
                for row in bundle.owned_credentials.values() {
                    present[CREDENTIALS].insert(key(CREDENTIALS, row)?);
                }
            } else {
                let row = local_rows[table]
                    .get(&id)
                    .or_else(|| remote_rows[table].get(&id))
                    .ok_or_else(invalid)?;
                result[table].push((*row).clone());
                present[table].insert(id);
            }
            changed = true;
        }
        // 依赖闭包按实际深度收敛，不以表数量截断合法的深层分组。
        if !changed {
            return Ok(());
        }
    }
}

fn check_trust(local: &Data, remote: &Data) -> Result<()> {
    let mut endpoints = HashMap::new();
    for row in local[TRUST].iter().chain(&remote[TRUST]) {
        let endpoint = (row[1].text()?, row[2].integer()?);
        let material = (row[3].text()?, row[4].blob()?);
        if endpoints
            .insert(endpoint, material)
            .is_some_and(|old| old != material)
        {
            // 先比较两端原始 pin，避免同一个记录 ID 的公钥修改被普通冲突规则吞掉。
            return Err(StorageError::Validation(
                "云端与本地主机信任冲突，请先在本地信任管理中核实并撤销旧记录",
            ));
        }
    }
    Ok(())
}

fn sort_data(data: &mut Data) -> Result<()> {
    for (index, table) in data.iter_mut().enumerate() {
        let mut keyed = std::mem::take(table)
            .into_iter()
            .map(|row| Ok((key(index, &row)?, row)))
            .collect::<Result<Vec<_>>>()?;
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        *table = keyed.into_iter().map(|(_, row)| row).collect();
    }
    Ok(())
}

fn normalize_tags(data: &mut Data, protected: &ProtectedReferences) -> Result<()> {
    let mut names = HashMap::<String, String>::new();
    let mut aliases = HashMap::new();
    // 本机专属配置的标签优先保留 ID，其余按稳定 ID 去重，避免误删本机关联。
    let mut tags = std::mem::take(&mut data[TAGS]);
    tags.sort_by_key(|row| !protected.tags.contains(row[0].text().unwrap_or_default()));
    for row in tags {
        let id = row[0].text()?.to_owned();
        let name = row[1].text()?.to_owned();
        if let Some(existing) = names.get(&name) {
            if protected.tags.contains(&id) {
                return Err(invalid());
            }
            aliases.insert(id, existing.clone());
        } else {
            names.insert(name, id);
            data[TAGS].push(row);
        }
    }
    for row in &mut data[LINKS] {
        if let Some(id) = aliases.get(row[1].text()?) {
            row[1].set_text(id.clone());
        }
    }
    let mut seen = HashSet::new();
    let mut links = Vec::new();
    for row in std::mem::take(&mut data[LINKS]) {
        if seen.insert(key(LINKS, &row)?) {
            links.push(row);
        }
    }
    data[LINKS] = links;
    Ok(())
}

fn normalize_profile_names(data: &mut Data) -> Result<()> {
    let reserved = data[PROFILES]
        .iter()
        .map(|row| Ok(row[2].text()?.to_owned()))
        .collect::<Result<HashSet<_>>>()?;
    let mut claimed = HashSet::new();
    for row in &mut data[PROFILES] {
        let name = row[2].text()?.to_owned();
        if claimed.insert(name.clone()) {
            continue;
        }
        let short_id = row[0].text()?.chars().take(8).collect::<String>();
        for number in 1.. {
            let suffix = format!(" ({short_id}-{number})");
            let candidate = format!(
                "{}{}",
                name.chars()
                    .take(128 - suffix.chars().count())
                    .collect::<String>(),
                suffix,
            );
            if !reserved.contains(&candidate) && claimed.insert(candidate.clone()) {
                row[2].set_text(candidate);
                break;
            }
        }
    }
    Ok(())
}

fn advance_revisions(data: &mut Data, local: &Data) -> Result<()> {
    let old_bundles = bundles(local)?;
    let new_bundles = bundles(data)?;
    let changed_connections = new_bundles
        .iter()
        .filter_map(|(id, next)| {
            old_bundles.get(id).and_then(|old| {
                (old.ssh != next.ssh
                    || old.auth != next.auth
                    || old.owned_credentials != next.owned_credentials)
                    .then(|| id.clone())
            })
        })
        .collect::<HashSet<_>>();
    drop(new_bundles);
    for (table, revision) in [(GROUPS, 4), (PROFILES, 7), (FORWARDS, 4), (SNIPPETS, 3)] {
        let old = rows(table, local)?;
        for row in &mut data[table] {
            let Some(previous) = old.get(row[0].text()?) else {
                continue;
            };
            let maximum = row[revision].integer()?.max(previous[revision].integer()?);
            // 仅业务字段变化提高本机 revision；计数/时间传播不再次递增，避免设备间循环上传。
            let changed = row[..revision] != previous[..revision]
                || (table == PROFILES && changed_connections.contains(row[0].text()?));
            row[revision] = Cell::Integer(if changed {
                maximum.checked_add(1).ok_or_else(invalid)?
            } else {
                maximum
            });
        }
    }
    Ok(())
}

fn merge(
    base: &Data,
    local: &Data,
    remote: &Data,
    protected: &ProtectedReferences,
) -> Result<Data> {
    check_trust(local, remote)?;
    let mut result: Data = TABLES.iter().map(|_| Vec::new()).collect();
    for index in [GROUPS, CREDENTIALS, TAGS, LINKS, FORWARDS, TRUST, SNIPPETS] {
        let before = rows(index, base)?;
        let here = rows(index, local)?;
        let there = rows(index, remote)?;
        let ids = before
            .keys()
            .chain(here.keys())
            .chain(there.keys())
            .collect::<BTreeSet<_>>();
        for id in ids {
            if let Some(row) = select(
                before.get(id).copied(),
                here.get(id).copied(),
                there.get(id).copied(),
                |left, right| business_row_equal(index, left, right),
            ) {
                if index != CREDENTIALS || row[3].optional_text()?.is_none() {
                    result[index].push(row.clone());
                }
            }
        }
    }
    let before = bundles(base)?;
    let here = bundles(local)?;
    let there = bundles(remote)?;
    let ids = before
        .keys()
        .chain(here.keys())
        .chain(there.keys())
        .collect::<BTreeSet<_>>();
    for id in ids {
        if let Some(bundle) = select(
            before.get(id),
            here.get(id),
            there.get(id),
            |left, right| {
                business_row_equal(PROFILES, left.profile, right.profile)
                    && left.ssh == right.ssh
                    && left.auth == right.auth
                    && left.owned_credentials == right.owned_credentials
            },
        ) {
            append_bundle(&mut result, bundle);
        }
    }
    restore_references(&mut result, local, remote, protected)?;
    sort_data(&mut result)?;
    normalize_tags(&mut result, protected)?;
    normalize_profile_names(&mut result)?;
    // 同端点同公钥只保留确定的一条，信任差异已在合并前拒绝。
    let mut endpoints = HashSet::new();
    let mut trusted = Vec::new();
    for row in std::mem::take(&mut result[TRUST]) {
        if endpoints.insert((row[1].text()?.to_owned(), row[2].integer()?)) {
            trusted.push(row);
        }
    }
    result[TRUST] = trusted;
    advance_revisions(&mut result, local)?;
    sort_data(&mut result)?;
    backup::validate(&result)?;
    Ok(result)
}

fn decode(database: Zeroizing<Vec<u8>>) -> Result<Data> {
    let mut data = backup_sqlite::decode(database)?;
    backup::validate(&data)?;
    for row in &mut data[PROFILES] {
        backup::strip_detection_token(row)?;
    }
    Ok(data)
}

fn equivalent(left: &Data, right: &Data) -> Result<bool> {
    for index in 0..TABLES.len() {
        if rows(index, left)? != rows(index, right)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// 按稳定行键比较业务快照，忽略 SQLite 页布局、行顺序、本机 revision 与系统识别令牌。
/// 调用方应在阻塞任务执行解析；本函数不打开活动数据库或运行快照中的 SQL。
pub fn equivalent_official_databases(left: &[u8], right: &[u8]) -> Result<bool> {
    let left = decode(Zeroizing::new(left.to_vec()))?;
    let right = decode(Zeroizing::new(right.to_vec()))?;
    for index in 0..TABLES.len() {
        let left = rows(index, &left)?;
        let right = rows(index, &right)?;
        if left.len() != right.len()
            || left.iter().any(|(id, row)| {
                right
                    .get(id)
                    .is_none_or(|other| !business_row_equal(index, row, other))
            })
        {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn delete_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    index: usize,
    row: &[Cell],
) -> Result<()> {
    let table = &TABLES[index];
    let columns = table.columns();
    let keys = if index == LINKS { 2 } else { 1 };
    let sql = format!(
        "DELETE FROM {} WHERE {}",
        table.name,
        columns[..keys]
            .iter()
            .map(|name| format!("{name}=?"))
            .collect::<Vec<_>>()
            .join(" AND ")
    );
    let mut query = sqlx::query(&sql);
    for cell in &row[..keys] {
        query = query.bind(cell.text()?);
    }
    query.execute(&mut **tx).await?;
    Ok(())
}

async fn update_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    index: usize,
    row: &[Cell],
) -> Result<()> {
    let table = &TABLES[index];
    let columns = table.columns();
    let sql = format!(
        "UPDATE {} SET {} WHERE {}=?",
        table.name,
        columns[1..]
            .iter()
            .map(|name| format!("{name}=?"))
            .collect::<Vec<_>>()
            .join(","),
        columns[0]
    );
    let mut query = sqlx::query(&sql);
    for cell in row[1..].iter().chain(&row[..1]) {
        query = match cell {
            Cell::Null => query.bind(Option::<String>::None),
            Cell::Integer(value) => query.bind(*value),
            Cell::Text(value) => query.bind(value.as_str()),
            Cell::Blob(value) => query.bind(value.as_slice()),
        };
    }
    query.execute(&mut **tx).await?;
    Ok(())
}

async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    local: &Data,
    merged: &Data,
) -> Result<()> {
    let before = (0..TABLES.len())
        .map(|index| rows(index, local))
        .collect::<Result<Vec<_>>>()?;
    let after = (0..TABLES.len())
        .map(|index| rows(index, merged))
        .collect::<Result<Vec<_>>>()?;
    // 仅移除有变化的白名单子行，保留本机专属配置的关联及未变化的信任/日志。
    for index in [LINKS, AUTH, SSH, FORWARDS, TRUST, SNIPPETS] {
        for (id, row) in &before[index] {
            if after[index].get(id).copied() != Some(*row) {
                delete_row(tx, index, row).await?;
            }
        }
    }
    for index in [PROFILES, CREDENTIALS, TAGS] {
        for (id, row) in &before[index] {
            if !after[index].contains_key(id) {
                delete_row(tx, index, row).await?;
            }
        }
    }
    for (id, row) in &before[GROUPS] {
        if after[GROUPS].get(id).copied() != Some(*row) {
            sqlx::query("UPDATE host_groups SET parent_id=NULL WHERE id=?")
                .bind(row[0].text()?)
                .execute(&mut **tx)
                .await?;
        }
    }
    for (id, row) in &before[GROUPS] {
        if !after[GROUPS].contains_key(id) {
            delete_row(tx, GROUPS, row).await?;
        }
    }
    // 先为名称交换让位，临时值显式避开所有旧名与新名，不触碰非白名单配置。
    for (index, column) in [(PROFILES, 2), (TAGS, 1)] {
        let mut reserved = local[index]
            .iter()
            .chain(&merged[index])
            .map(|row| Ok(row[column].text()?.to_owned()))
            .collect::<Result<HashSet<_>>>()?;
        for (id, row) in &before[index] {
            if let Some(next) = after[index].get(id) {
                if row[column] == next[column] {
                    continue;
                }
                let temporary = loop {
                    let candidate = format!("sync-{}", uuid::Uuid::new_v4());
                    if reserved.insert(candidate.clone()) {
                        break candidate;
                    }
                };
                sqlx::query(&format!(
                    "UPDATE {} SET name=? WHERE id=?",
                    TABLES[index].name
                ))
                .bind(temporary)
                .bind(id)
                .execute(&mut **tx)
                .await?;
            }
        }
    }
    for (index, table) in TABLES.iter().enumerate() {
        for row in &merged[index] {
            let id = key(index, row)?;
            let old = before[index].get(&id).copied();
            if old == Some(row) {
                continue;
            }
            if [GROUPS, PROFILES, CREDENTIALS, TAGS].contains(&index) && old.is_some() {
                let mut stored = row.clone();
                if index == PROFILES {
                    let old = old.ok_or_else(invalid)?;
                    if old[6] == row[6] && before[SSH].get(&id) == after[SSH].get(&id) {
                        // 只同步名称/分组等元数据时，不撤销本机尚在执行的系统识别请求。
                        let config: String = sqlx::query_scalar(
                            "SELECT config_json FROM connection_profiles WHERE id=?",
                        )
                        .bind(&id)
                        .fetch_one(&mut **tx)
                        .await?;
                        stored[6] = Cell::Text(Zeroizing::new(config));
                    }
                }
                update_row(tx, index, &stored).await?;
            } else {
                backup::insert(tx, table, row).await?;
            }
        }
    }
    if !sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&mut **tx)
        .await?
        .is_empty()
    {
        return Err(invalid());
    }
    Ok(())
}

/// 已读取并成功合并的云端头，必须与本地合并结果在同一事务记录。
#[derive(Clone, Copy)]
pub struct OfficialSyncObservation<'a> {
    pub origin: &'a str,
    pub account_id: &'a str,
    pub revision: i64,
}

impl SqliteStore {
    pub async fn official_baseline(
        &self,
        origin: &str,
        account: &str,
    ) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let bytes: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT baseline FROM official_sync_state WHERE origin=? AND account_id=?",
        )
        .bind(origin)
        .bind(account)
        .fetch_optional(&*self.pool().await?)
        .await?;
        Ok(bytes.map(Zeroizing::new))
    }

    pub async fn save_official_baseline(
        &self,
        origin: &str,
        account: &str,
        revision: i64,
        database: &[u8],
    ) -> Result<()> {
        if database.len() > backup::MAX_DATABASE_BYTES || revision < 0 {
            return Err(StorageError::Validation("同步基线或版本无效"));
        }
        sqlx::query("INSERT INTO official_sync_state(origin,account_id,revision,baseline) VALUES(?,?,?,?) ON CONFLICT(origin,account_id) DO UPDATE SET revision=excluded.revision,baseline=excluded.baseline").bind(origin).bind(account).bind(revision).bind(database).execute(&*self.pool().await?).await?;
        Ok(())
    }

    pub async fn merge_official_database(
        &self,
        database: Zeroizing<Vec<u8>>,
        baseline: Option<Zeroizing<Vec<u8>>>,
        observation: Option<OfficialSyncObservation<'_>>,
    ) -> Result<Zeroizing<Vec<u8>>> {
        if observation.is_some_and(|head| head.revision <= 0) {
            return Err(StorageError::Validation("已观测云端同步版本无效"));
        }
        let observed_database = observation.map(|_| database.clone());
        let (remote, base) = tokio::task::spawn_blocking(move || {
            let remote = decode(database)?;
            let base = if let Some(bytes) = baseline {
                decode(bytes)?
            } else {
                TABLES.iter().map(|_| Vec::new()).collect()
            };
            Ok::<_, StorageError>((remote, base))
        })
        .await
        .map_err(|_| StorageError::Validation("同步解析任务中断"))??;
        let pool = self.pool().await?;
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("PRAGMA defer_foreign_keys=ON")
            .execute(&mut *tx)
            .await?;
        let local = backup::read_data(&mut tx).await?;
        backup::validate(&local)?;
        let protected = ProtectedReferences {
            groups: sqlx::query_scalar::<_, String>("SELECT group_id FROM connection_profiles WHERE kind NOT IN ('ssh','telnet','serial','workspace') AND group_id IS NOT NULL").fetch_all(&mut *tx).await?.into_iter().collect(),
            tags: sqlx::query_scalar::<_, String>("SELECT pt.tag_id FROM profile_tags pt JOIN connection_profiles p ON p.id=pt.profile_id WHERE p.kind NOT IN ('ssh','telnet','serial','workspace')").fetch_all(&mut *tx).await?.into_iter().collect(),
        };
        let (local, merged, bytes, changed) = tokio::task::spawn_blocking(move || {
            let merged = merge(&base, &local, &remote, &protected)?;
            let changed = !equivalent(&local, &merged)?;
            let bytes = backup_sqlite::encode(&merged)?;
            Ok::<_, StorageError>((local, merged, bytes, changed))
        })
        .await
        .map_err(|_| StorageError::Validation("同步合并任务中断"))??;
        // 空轮询不写业务表，亦不消耗本机系统识别令牌或触发外键副作用。
        if changed {
            apply(&mut tx, &local, &merged).await?;
        }
        if let (Some(head), Some(database)) = (observation, observed_database) {
            // 即使后续 PUT 冲突、响应丢失或进程退出，下次也以真正吸收过的远端版本为基线。
            // 同一云版本不可变，重复轮询不重写大块基线；不能在业务提交后单独保存此记录。
            sqlx::query("INSERT INTO official_sync_state(origin,account_id,revision,baseline) VALUES(?,?,?,?) ON CONFLICT(origin,account_id) DO UPDATE SET revision=excluded.revision,baseline=excluded.baseline WHERE official_sync_state.revision<>excluded.revision")
                .bind(head.origin).bind(head.account_id).bind(head.revision).bind(database.as_slice())
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(bytes)
    }
}

#[cfg(test)]
#[path = "official_sync_tests.rs"]
mod tests;

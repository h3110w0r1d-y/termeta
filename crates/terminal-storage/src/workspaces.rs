use crate::*;
use sqlx::Row;

// 工作区写入沿用保存项的兼容表示，两个保存入口共用空协议字段。
fn workspace_profile(
    id: String,
    revision: i64,
    name: String,
    group_id: Option<String>,
    sort_order: i64,
    workspace: SavedWorkspace,
) -> HostProfile {
    HostProfile {
        id,
        revision,
        name,
        group_id,
        sort_order,
        workspace: Some(workspace),
        kind: "workspace".into(),
        startup: terminal_core::SshStartup::default(),
        serial_config: None,
        icon: "ssh".into(),
        system_detection_attempted: false,
        address: String::new(),
        port: 0,
        username: String::new(),
        tags: vec![],
        auth_plan: vec![],
        has_password: false,
    }
}

pub(crate) async fn ensure_unique_profile_name(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    name: &str,
    except: Option<&str>,
) -> Result<()> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM connection_profiles WHERE name=? AND kind IN ('ssh','telnet','serial','workspace') AND (? IS NULL OR id<>?))")
        .bind(name).bind(except).bind(except).fetch_one(&mut **tx).await?;
    if exists {
        return Err(StorageError::Validation(
            "名称已存在，SSH、Telnet、串口与工作区名称不能重复",
        ));
    }
    Ok(())
}
impl SqliteStore {
    pub async fn save_workspace(&self, draft: WorkspaceDraft) -> Result<HostProfile> {
        validate_name(&draft.name)?;
        let mut references = Vec::new();
        validate_layout(&draft.workspace.root, 0, &mut references)?;
        if references.is_empty() || draft.workspace.focused_index >= references.len() {
            return Err(StorageError::Validation("工作区焦点无效"));
        }
        let mut tx = self.pool().await?.begin_with("BEGIN IMMEDIATE").await?;
        for reference in &references {
            let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM connection_profiles WHERE id=? AND kind IN ('ssh','telnet','serial'))")
                .bind(reference).fetch_one(&mut *tx).await?;
            if !valid {
                return Err(StorageError::Validation(
                    "工作区引用的连接已删除或类型无效，请先保存连接配置",
                ));
            }
        }
        let existing = sqlx::query("SELECT id,kind,revision,group_id FROM connection_profiles WHERE name=? AND kind IN ('ssh','telnet','serial','workspace')")
            .bind(draft.name.trim()).fetch_optional(&mut *tx).await?;
        let config = serde_json::to_string(&serde_json::json!({"workspace": draft.workspace}))?;
        let (id, revision, group_id) = if let Some(row) = existing {
            if row.try_get::<String, _>("kind")? != "workspace" {
                return Err(StorageError::Validation(
                    "名称已被连接使用，只允许替换同名工作区",
                ));
            }
            let revision: i64 = row.try_get("revision")?;
            if draft.expected_revision != Some(revision) {
                return Err(StorageError::Conflict);
            }
            let id: String = row.try_get("id")?;
            sqlx::query("UPDATE connection_profiles SET config_json=?,revision=revision+1,updated_at=? WHERE id=?")
                .bind(&config).bind(now()).bind(&id).execute(&mut *tx).await?;
            (
                id,
                revision + 1,
                row.try_get::<Option<String>, _>("group_id")?,
            )
        } else {
            if draft.expected_revision.is_some() {
                return Err(StorageError::Conflict);
            }
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO connection_profiles(id,kind,name,favorite,config_json,created_at,updated_at) VALUES(?,'workspace',?,0,?,?,?)")
                .bind(&id).bind(draft.name.trim()).bind(&config).bind(now()).bind(now()).execute(&mut *tx).await?;
            (id, 1, None)
        };
        let sort_order =
            sqlx::query_scalar("SELECT sort_order FROM connection_profiles WHERE id=?")
                .bind(&id)
                .fetch_one(&mut *tx)
                .await?;
        let saved = workspace_profile(
            id,
            revision,
            draft.name.trim().into(),
            group_id,
            sort_order,
            draft.workspace,
        );
        tx.commit().await?;
        Ok(saved)
    }
    /// 只开放分组，名称和布局必须通过保存工作区入口整体替换。
    pub(crate) async fn update_workspace_metadata(&self, draft: HostDraft) -> Result<HostProfile> {
        let id = draft
            .id
            .as_deref()
            .ok_or(StorageError::Validation("工作区只能从 Workspace 标签保存"))?;
        let mut tx = self.pool().await?.begin_with("BEGIN IMMEDIATE").await?;
        let row = sqlx::query("SELECT name,revision,config_json FROM connection_profiles WHERE id=? AND kind='workspace'")
            .bind(id).fetch_optional(&mut *tx).await?.ok_or(StorageError::NotFound)?;
        let revision: i64 = row.try_get("revision")?;
        if draft.revision != Some(revision) {
            return Err(StorageError::Conflict);
        }
        if draft.name != row.try_get::<String, _>("name")?
            || !draft.auth_plan.is_empty()
            || draft.serial_config.is_some()
            || !draft.address.is_empty()
            || draft.port != 0
            || !draft.username.is_empty()
            || !draft.tags.is_empty()
            || draft.startup != terminal_core::SshStartup::Shell
        {
            return Err(StorageError::Validation(
                "工作区不能编辑，请重新保存进行替换",
            ));
        }
        let config: serde_json::Value = serde_json::from_str(row.try_get("config_json")?)?;
        let workspace = serde_json::from_value(config["workspace"].clone())?;
        sqlx::query(
            "UPDATE connection_profiles SET group_id=?,revision=revision+1,updated_at=? WHERE id=?",
        )
        .bind(&draft.group_id)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        let sort_order =
            sqlx::query_scalar("SELECT sort_order FROM connection_profiles WHERE id=?")
                .bind(&id)
                .fetch_one(&mut *tx)
                .await?;
        let saved = workspace_profile(
            id.into(),
            revision + 1,
            draft.name,
            draft.group_id,
            sort_order,
            workspace,
        );
        tx.commit().await?;
        Ok(saved)
    }
}
pub(crate) fn validate_layout(
    node: &SavedLayout,
    depth: usize,
    references: &mut Vec<String>,
) -> Result<()> {
    if depth > 32 || references.len() >= 64 {
        return Err(StorageError::Validation("工作区最多64个窗格、32层分割"));
    }
    match node {
        SavedLayout::Connection { profile_id } => {
            if profile_id.is_empty() || profile_id.len() > 128 {
                return Err(StorageError::Validation("工作区连接引用无效"));
            }
            references.push(profile_id.clone());
        }
        SavedLayout::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            if !matches!(axis.as_str(), "horizontal" | "vertical")
                || !ratio.is_finite()
                || !(0.1..=0.9).contains(ratio)
            {
                return Err(StorageError::Validation("工作区分割方向或比例无效"));
            }
            validate_layout(first, depth + 1, references)?;
            validate_layout(second, depth + 1, references)?;
        }
    }
    Ok(())
}

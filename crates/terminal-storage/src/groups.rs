use crate::*;
use sqlx::Row;
use uuid::Uuid;

impl SqliteStore {
    pub async fn list_groups(&self) -> Result<Vec<HostGroup>> {
        sqlx::query("SELECT * FROM host_groups ORDER BY sort_order,name COLLATE NOCASE")
            .fetch_all(self.pool()?)
            .await?
            .into_iter()
            .map(|r| {
                Ok(HostGroup {
                    id: r.try_get("id")?,
                    parent_id: r.try_get("parent_id")?,
                    name: r.try_get("name")?,
                    sort_order: r.try_get("sort_order")?,
                    revision: r.try_get("revision")?,
                })
            })
            .collect()
    }
    pub async fn save_group(&self, draft: GroupDraft) -> Result<HostGroup> {
        validate_name(&draft.name)?;
        let id = draft
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let mut tx = self.pool()?.begin_with("BEGIN IMMEDIATE").await?;
        let mut ancestor = draft.parent_id.clone();
        let mut visited = std::collections::HashSet::new();
        while let Some(parent) = ancestor {
            if parent == id || !visited.insert(parent.clone()) {
                return Err(StorageError::Validation("分组不能形成环"));
            }
            ancestor = sqlx::query_scalar::<_, Option<String>>(
                "SELECT parent_id FROM host_groups WHERE id=?",
            )
            .bind(parent)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound)?;
        }
        if draft.id.is_some() {
            let result=sqlx::query("UPDATE host_groups SET parent_id=?,name=?,sort_order=?,revision=revision+1,updated_at=? WHERE id=? AND revision=?").bind(&draft.parent_id).bind(draft.name.trim()).bind(draft.sort_order).bind(now()).bind(&id).bind(draft.revision.ok_or(StorageError::Conflict)?).execute(&mut *tx).await?;
            if result.rows_affected() != 1 {
                return Err(StorageError::Conflict);
            }
        } else {
            sqlx::query("INSERT INTO host_groups(id,parent_id,name,sort_order,created_at,updated_at) VALUES(?,?,?,?,?,?)").bind(&id).bind(&draft.parent_id).bind(draft.name.trim()).bind(draft.sort_order).bind(now()).bind(now()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        self.list_groups()
            .await?
            .into_iter()
            .find(|g| g.id == id)
            .ok_or(StorageError::NotFound)
    }
    pub async fn delete_group(&self, id: &str, revision: i64) -> Result<()> {
        let mut tx = self.pool()?.begin_with("BEGIN IMMEDIATE").await?;
        let parent = sqlx::query_scalar::<_, Option<String>>(
            "SELECT parent_id FROM host_groups WHERE id=? AND revision=?",
        )
        .bind(id)
        .bind(revision)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(StorageError::Conflict)?;
        // 删除分组只把直属子项上移，现有会话及共享凭据不受影响。
        sqlx::query(
            "UPDATE host_groups SET parent_id=?,revision=revision+1,updated_at=? WHERE parent_id=?",
        )
        .bind(&parent)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE connection_profiles SET group_id=?,revision=revision+1,updated_at=? WHERE group_id=?").bind(parent).bind(now()).bind(id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM host_groups WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

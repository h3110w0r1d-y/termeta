use crate::*;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use terminal_core::ForwardRule;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForwardDraft {
    pub id: Option<String>,
    pub name: String,
    pub profile_id: String,
    pub rule: ForwardRule,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardProfile {
    pub id: String,
    pub name: String,
    pub profile_id: Option<String>,
    pub rule: ForwardRule,
    pub revision: i64,
}
fn decode(row: sqlx::sqlite::SqliteRow) -> Result<ForwardProfile> {
    Ok(ForwardProfile {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        profile_id: row.try_get("profile_id")?,
        rule: serde_json::from_str(row.try_get("rule_json")?)?,
        revision: row.try_get("revision")?,
    })
}
impl SqliteStore {
    pub async fn list_forwards(&self) -> Result<Vec<ForwardProfile>> {
        sqlx::query("SELECT * FROM port_forwards ORDER BY name,id")
            .fetch_all(&*self.pool().await?)
            .await?
            .into_iter()
            .map(decode)
            .collect()
    }
    pub async fn get_forward(&self, id: &str) -> Result<ForwardProfile> {
        decode(
            sqlx::query("SELECT * FROM port_forwards WHERE id=?")
                .bind(id)
                .fetch_optional(&*self.pool().await?)
                .await?
                .ok_or(StorageError::NotFound)?,
        )
    }
    pub async fn save_forward(
        &self,
        draft: ForwardDraft,
        revision: Option<i64>,
    ) -> Result<ForwardProfile> {
        validate_name(&draft.name)?;
        draft
            .rule
            .validate()
            .map_err(|_| StorageError::Validation("端口转发配置无效"))?;
        let mut tx = self.pool().await?.begin_with("BEGIN IMMEDIATE").await?;
        let kind: Option<String> =
            sqlx::query_scalar("SELECT kind FROM connection_profiles WHERE id=?")
                .bind(&draft.profile_id)
                .fetch_optional(&mut *tx)
                .await?;
        if kind.as_deref() != Some("ssh") {
            return Err(StorageError::Validation(
                "端口转发必须引用已保存的 SSH 主机",
            ));
        }
        let encoded = serde_json::to_string(&draft.rule)?;
        let id = if let Some(id) = draft.id {
            let revision = revision.ok_or(StorageError::Conflict)?;
            if sqlx::query("UPDATE port_forwards SET name=?,profile_id=?,rule_json=?,revision=revision+1,updated_at=? WHERE id=? AND revision=?")
                .bind(draft.name.trim()).bind(&draft.profile_id).bind(encoded).bind(now()).bind(&id).bind(revision)
                .execute(&mut *tx).await?.rows_affected() != 1 { return Err(StorageError::Conflict); }
            id
        } else {
            if revision.is_some() {
                return Err(StorageError::Conflict);
            }
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO port_forwards(id,name,profile_id,rule_json,created_at,updated_at) VALUES(?,?,?,?,?,?)")
                .bind(&id).bind(draft.name.trim()).bind(&draft.profile_id).bind(encoded).bind(now()).bind(now())
                .execute(&mut *tx).await?;
            id
        };
        // 在提交前读取，避免已成功提交却因后续重读失败而向调用者报告保存失败。
        let saved = decode(
            sqlx::query("SELECT * FROM port_forwards WHERE id=?")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?,
        )?;
        tx.commit().await?;
        Ok(saved)
    }
    pub async fn delete_forward(&self, id: &str, revision: i64) -> Result<()> {
        if sqlx::query("DELETE FROM port_forwards WHERE id=? AND revision=?")
            .bind(id)
            .bind(revision)
            .execute(&*self.pool().await?)
            .await?
            .rows_affected()
            != 1
        {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }
}

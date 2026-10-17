//! 命令片段只负责管理正文，不隐式执行或关联活动终端。
use crate::{Result, SqliteStore, StorageError, now, validate_name};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection};

pub const MAX_SNIPPET_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub content: String,
    pub revision: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnippetDraft {
    pub id: Option<String>,
    pub revision: Option<i64>,
    pub name: String,
    pub content: String,
}

pub(crate) async fn ensure_snippets_schema(connection: &mut SqliteConnection) -> Result<()> {
    // 兼容初始化不改动已应用迁移，也不改变旧表或 SQLx 的校验记录。
    sqlx::query("CREATE TABLE IF NOT EXISTS snippets (id TEXT PRIMARY KEY NOT NULL,name TEXT NOT NULL,content TEXT NOT NULL,revision INTEGER NOT NULL CHECK(revision > 0),created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL)")
        .execute(connection).await?;
    Ok(())
}

pub(crate) fn validate_snippet(name: &str, content: &str) -> Result<()> {
    validate_name(name)?;
    if name != name.trim() {
        return Err(StorageError::Validation("片段名称不能包含首尾空白"));
    }
    if content.trim().is_empty() || content.len() > MAX_SNIPPET_BYTES || content.contains('\0') {
        return Err(StorageError::Validation(
            "片段正文不能为空、包含 NUL 或超过 64 KiB",
        ));
    }
    Ok(())
}

impl SqliteStore {
    pub async fn list_snippets(&self) -> Result<Vec<Snippet>> {
        let pool = self.pool().await?;
        Ok(sqlx::query_as("SELECT id,name,content,revision,created_at,updated_at FROM snippets ORDER BY updated_at DESC,id")
            .fetch_all(&*pool).await?)
    }
    pub async fn save_snippet(&self, draft: SnippetDraft) -> Result<Snippet> {
        let name = draft.name.trim();
        validate_snippet(name, &draft.content)?;
        let pool = self.pool().await?;
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        let timestamp = now();
        let id = match (draft.id, draft.revision) {
            (None, None) => {
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO snippets(id,name,content,revision,created_at,updated_at) VALUES(?,?,?,1,?,?)")
                    .bind(&id).bind(name).bind(&draft.content).bind(timestamp).bind(timestamp)
                    .execute(&mut *tx).await?;
                id
            }
            (Some(id), Some(revision)) if revision > 0 => {
                let changed = sqlx::query("UPDATE snippets SET name=?,content=?,revision=revision+1,updated_at=? WHERE id=? AND revision=?")
                    .bind(name).bind(&draft.content).bind(timestamp).bind(&id).bind(revision)
                    .execute(&mut *tx).await?.rows_affected();
                if changed != 1 {
                    return Err(StorageError::Conflict);
                }
                id
            }
            _ => return Err(StorageError::Validation("更新片段须同时提供标识和有效版本")),
        };
        let snippet = sqlx::query_as(
            "SELECT id,name,content,revision,created_at,updated_at FROM snippets WHERE id=?",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(snippet)
    }
    pub async fn delete_snippet(&self, id: &str, revision: i64) -> Result<()> {
        let pool = self.pool().await?;
        if sqlx::query("DELETE FROM snippets WHERE id=? AND revision=?")
            .bind(id)
            .bind(revision)
            .execute(&*pool)
            .await?
            .rows_affected()
            != 1
        {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snippet_validation_preserves_multiline_text_and_bounds_size() {
        assert!(validate_snippet("部署", "  echo 'hello'\n\n").is_ok());
        assert!(validate_snippet("部署", " \n ").is_err());
        assert!(validate_snippet("部署", "echo\0bad").is_err());
        assert!(validate_snippet("部署", &"a".repeat(MAX_SNIPPET_BYTES + 1)).is_err());
        assert!(validate_snippet(" 部署", "echo ok").is_err());
    }
    #[tokio::test(flavor = "current_thread")]
    async fn edits_and_deletes_reject_stale_revisions() {
        let path =
            std::env::temp_dir().join(format!("termeta-snippets-{}.sqlite3", uuid::Uuid::new_v4()));
        let store = SqliteStore::open(&path).await.unwrap();
        let saved = store
            .save_snippet(SnippetDraft {
                id: None,
                revision: None,
                name: " 命令 ".into(),
                content: "echo ok\n".into(),
            })
            .await
            .unwrap();
        assert_eq!(saved.name, "命令");
        assert_eq!(saved.content, "echo ok\n");
        let edited = store
            .save_snippet(SnippetDraft {
                id: Some(saved.id.clone()),
                revision: Some(saved.revision),
                name: saved.name.clone(),
                content: "echo updated".into(),
            })
            .await
            .unwrap();
        assert_eq!(edited.revision, saved.revision + 1);
        assert!(matches!(
            store.delete_snippet(&saved.id, saved.revision).await,
            Err(StorageError::Conflict)
        ));
        store
            .delete_snippet(&edited.id, edited.revision)
            .await
            .unwrap();
        assert!(store.list_snippets().await.unwrap().is_empty());
        drop(store);
        let _ = std::fs::remove_file(path);
    }
}

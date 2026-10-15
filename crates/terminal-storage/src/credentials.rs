use crate::*;
use sqlx::Row;
use uuid::Uuid;

impl SqliteStore {
    pub async fn list_credentials(&self) -> Result<Vec<CredentialRecord>> {
        sqlx::query("SELECT id,kind,label,status FROM credentials ORDER BY created_at")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|r| {
                Ok(CredentialRecord {
                    id: r.try_get("id")?,
                    kind: r.try_get("kind")?,
                    label: r.try_get("label")?,
                    status: r.try_get("status")?,
                })
            })
            .collect()
    }
    pub async fn get_credential(&self, id: &str) -> Result<CredentialRecord> {
        self.list_credentials()
            .await?
            .into_iter()
            .find(|r| r.id == id)
            .ok_or(StorageError::NotFound)
    }
    pub async fn begin_credential(&self, kind: &str, label: &str) -> Result<String> {
        if !matches!(kind, "password" | "passphrase") {
            return Err(StorageError::Validation("凭据类型无效"));
        }
        validate_name(label)?;
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO credentials(id,kind,label,store_account,status,created_at,updated_at) VALUES(?,?,?,?,'pending',?,?)").bind(&id).bind(kind).bind(label).bind(&id).bind(now()).bind(now()).execute(&self.pool).await?;
        Ok(id)
    }
    pub async fn credential_ready(&self, id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE credentials SET status='ready',updated_at=? WHERE id=? AND status='pending'",
        )
        .bind(now())
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub async fn begin_delete_credential(&self, id: &str) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let references:i64=sqlx::query_scalar("SELECT COUNT(*) FROM profile_auth_steps WHERE credential_id=? OR passphrase_credential_id=?").bind(id).bind(id).fetch_one(&mut *tx).await?;
        if references > 0 {
            return Err(StorageError::Validation("凭据仍被主机引用，请先解绑"));
        }
        let changed =
            sqlx::query("UPDATE credentials SET status='deleting',updated_at=? WHERE id=?")
                .bind(now())
                .bind(id)
                .execute(&mut *tx)
                .await?;
        if changed.rows_affected() != 1 {
            return Err(StorageError::NotFound);
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn finish_delete_credential(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM credentials WHERE id=? AND status='deleting'")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

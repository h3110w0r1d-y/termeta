use crate::*;
use sqlx::Row;
use uuid::Uuid;

fn credential_from_row(row: sqlx::sqlite::SqliteRow) -> Result<CredentialRecord> {
    Ok(CredentialRecord {
        id: row.try_get("id")?,
        owner_profile_id: row.try_get("owner_profile_id")?,
        kind: row.try_get("kind")?,
        label: row.try_get("label")?,
        status: row.try_get("status")?,
    })
}

impl SqliteStore {
    /// 公共凭据的元数据、秘密和就绪状态在同一条 SQL 中提交，不留下两阶段占位项。
    pub async fn save_public_credential(
        &self,
        kind: &str,
        label: &str,
        secret: &str,
    ) -> Result<String> {
        if !matches!(kind, "password" | "privateKey") || secret.is_empty() {
            return Err(StorageError::Validation("凭据类型或秘密无效"));
        }
        validate_name(label)?;
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO credentials(id,kind,label,owner_profile_id,secret,status,created_at,updated_at) VALUES(?,?,?,NULL,?,'ready',?,?)")
            .bind(&id).bind(kind).bind(label.trim()).bind(secret).bind(now()).bind(now())
            .execute(&*self.pool().await?).await?;
        Ok(id)
    }

    /// 公共列表不暴露主机专属密码的元数据与定位符。
    pub async fn list_credentials(&self) -> Result<Vec<CredentialRecord>> {
        sqlx::query("SELECT id,owner_profile_id,kind,label,status FROM credentials WHERE owner_profile_id IS NULL ORDER BY created_at")
            .fetch_all(&*self.pool().await?).await?.into_iter().map(credential_from_row).collect()
    }
    /// 仅供凭据协调器恢复两阶段写入与删除，不直接暴露到前端。
    pub async fn list_all_credentials(&self) -> Result<Vec<CredentialRecord>> {
        sqlx::query(
            "SELECT id,owner_profile_id,kind,label,status FROM credentials ORDER BY created_at",
        )
        .fetch_all(&*self.pool().await?)
        .await?
        .into_iter()
        .map(credential_from_row)
        .collect()
    }
    pub async fn get_credential(&self, id: &str) -> Result<CredentialRecord> {
        let row =
            sqlx::query("SELECT id,owner_profile_id,kind,label,status FROM credentials WHERE id=?")
                .bind(id)
                .fetch_optional(&*self.pool().await?)
                .await?
                .ok_or(StorageError::NotFound)?;
        credential_from_row(row)
    }
    pub async fn begin_host_password(&self, profile_id: &str, label: &str) -> Result<String> {
        if Uuid::parse_str(profile_id).is_err() {
            return Err(StorageError::Validation("主机标识无效"));
        }
        self.begin_scoped_credential("password", label, Some(profile_id))
            .await
    }
    async fn begin_scoped_credential(
        &self,
        kind: &str,
        label: &str,
        owner: Option<&str>,
    ) -> Result<String> {
        if !matches!(kind, "password" | "privateKey") {
            return Err(StorageError::Validation("凭据类型无效"));
        }
        validate_name(label)?;
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO credentials(id,kind,label,owner_profile_id,status,created_at,updated_at) VALUES(?,?,?,?,'pending',?,?)")
            .bind(&id).bind(kind).bind(label.trim()).bind(owner).bind(now()).bind(now()).execute(&*self.pool().await?).await?;
        Ok(id)
    }
    pub async fn begin_delete_credential(&self, id: &str) -> Result<()> {
        let mut tx = self.pool().await?.begin_with("BEGIN IMMEDIATE").await?;
        let references: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM profile_auth_steps WHERE credential_id=?")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
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
    /// 操作锁排除在途写入；历史公共 pending 与主机孤立项只在没有引用时回收。
    pub async fn mark_orphaned_credentials_deleting(&self) -> Result<()> {
        sqlx::query("UPDATE credentials SET status='deleting',updated_at=? WHERE ((owner_profile_id IS NOT NULL AND status IN ('pending','ready')) OR (owner_profile_id IS NULL AND status='pending')) AND NOT EXISTS(SELECT 1 FROM profile_auth_steps a WHERE a.credential_id=credentials.id)")
            .bind(now()).execute(&*self.pool().await?).await?;
        Ok(())
    }
    pub async fn finish_delete_credential(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM credentials WHERE id=? AND status='deleting'")
            .bind(id)
            .execute(&*self.pool().await?)
            .await?;
        Ok(())
    }
}

impl SqliteStore {
    /// 秘密仅在已解锁的 SQLCipher 库内保存，不返回给公共凭据 DTO。
    pub async fn set_credential_secret(&self, id: &str, secret: &str) -> Result<()> {
        let changed = sqlx::query("UPDATE credentials SET secret=?,status='ready',updated_at=? WHERE id=? AND status='pending'")
            .bind(secret).bind(now()).bind(id).execute(&*self.pool().await?).await?;
        if changed.rows_affected() != 1 {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }
    pub async fn credential_secret(&self, id: &str) -> Result<zeroize::Zeroizing<String>> {
        let secret: Option<String> = sqlx::query_scalar(
            "SELECT secret FROM credentials WHERE id=? AND status IN ('pending','ready')",
        )
        .bind(id)
        .fetch_optional(&*self.pool().await?)
        .await?
        .flatten();
        secret
            .map(zeroize::Zeroizing::new)
            .ok_or(StorageError::NotFound)
    }
    pub async fn erase_credential_secret(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE credentials SET secret=NULL WHERE id=? AND status='deleting'")
            .bind(id)
            .execute(&*self.pool().await?)
            .await?;
        Ok(())
    }
}

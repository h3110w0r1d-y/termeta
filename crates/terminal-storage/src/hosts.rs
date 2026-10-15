use crate::*;
use sqlx::Row;
use terminal_core::{AuthStep, SshConfig};
use uuid::Uuid;

impl SqliteStore {
    pub async fn list_hosts(&self, search: &str) -> Result<Vec<HostProfile>> {
        let rows = sqlx::query("SELECT p.*,s.address,s.port,s.username FROM connection_profiles p JOIN ssh_profiles s ON s.profile_id=p.id ORDER BY p.favorite DESC,p.name COLLATE NOCASE")
            .fetch_all(&self.pool).await?;
        let mut profiles = Vec::with_capacity(rows.len());
        let search = search.to_lowercase();
        for row in rows {
            let id: String = row.try_get("id")?;
            let tags = sqlx::query_scalar::<_, String>("SELECT t.name FROM tags t JOIN profile_tags p ON p.tag_id=t.id WHERE p.profile_id=? ORDER BY t.name").bind(&id).fetch_all(&self.pool).await?;
            let auth = sqlx::query_scalar::<_, String>(
                "SELECT step_json FROM profile_auth_steps WHERE profile_id=? ORDER BY sort_order",
            )
            .bind(&id)
            .fetch_all(&self.pool)
            .await?;
            let host = HostProfile {
                id,
                revision: row.try_get("revision")?,
                name: row.try_get("name")?,
                address: row.try_get("address")?,
                port: row.try_get::<i64, _>("port")? as u16,
                username: row.try_get("username")?,
                group_id: row.try_get("group_id")?,
                favorite: row.try_get("favorite")?,
                tags,
                auth_plan: auth
                    .iter()
                    .map(|a| serde_json::from_str(a))
                    .collect::<std::result::Result<_, _>>()?,
            };
            if search.is_empty()
                || [&host.name, &host.address, &host.username]
                    .iter()
                    .any(|s| s.to_lowercase().contains(&search))
                || host.tags.iter().any(|t| t.to_lowercase().contains(&search))
            {
                profiles.push(host);
            }
        }
        Ok(profiles)
    }
    pub async fn get_host(&self, id: &str) -> Result<HostProfile> {
        self.list_hosts("")
            .await?
            .into_iter()
            .find(|h| h.id == id)
            .ok_or(StorageError::NotFound)
    }
    pub async fn save_host(&self, draft: HostDraft) -> Result<HostProfile> {
        validate_name(&draft.name)?;
        SshConfig {
            config_version: 1,
            address: draft.address.clone(),
            port: draft.port,
            username: draft.username.clone(),
            auth_plan: draft.auth_plan.clone(),
        }
        .validate()
        .map_err(|_| StorageError::Validation("SSH 配置无效"))?;
        if draft.tags.len() > 32
            || draft.tags.iter().any(|t| {
                t.trim().is_empty() || t.chars().count() > 64 || t.chars().any(char::is_control)
            })
        {
            return Err(StorageError::Validation("标签数量或长度无效"));
        }
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let id = draft
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        if draft.id.is_some() {
            let revision = draft.revision.ok_or(StorageError::Conflict)?;
            let result=sqlx::query("UPDATE connection_profiles SET name=?,group_id=?,favorite=?,revision=revision+1,updated_at=? WHERE id=? AND revision=?")
                .bind(draft.name.trim()).bind(&draft.group_id).bind(draft.favorite).bind(now()).bind(&id).bind(revision).execute(&mut *tx).await?;
            if result.rows_affected() != 1 {
                return Err(StorageError::Conflict);
            }
            sqlx::query("DELETE FROM profile_auth_steps WHERE profile_id=?")
                .bind(&id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM profile_tags WHERE profile_id=?")
                .bind(&id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query("INSERT INTO connection_profiles(id,name,group_id,favorite,created_at,updated_at) VALUES(?,?,?,?,?,?)").bind(&id).bind(draft.name.trim()).bind(&draft.group_id).bind(draft.favorite).bind(now()).bind(now()).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO ssh_profiles(profile_id,address,canonical_host,port,username) VALUES(?,?,?,?,?) ON CONFLICT(profile_id) DO UPDATE SET address=excluded.address,canonical_host=excluded.canonical_host,port=excluded.port,username=excluded.username")
            .bind(&id).bind(&draft.address).bind(canonical_host(&draft.address)).bind(i64::from(draft.port)).bind(&draft.username).execute(&mut *tx).await?;
        for (index, step) in draft.auth_plan.iter().enumerate() {
            let (method, credential, key_file, passphrase) = match step {
                AuthStep::Password { credential_id } => {
                    ("password", credential_id.as_deref(), None, None)
                }
                AuthStep::PrivateKey {
                    key_file_ref,
                    passphrase_credential_id,
                } => (
                    "privateKey",
                    None,
                    Some(key_file_ref.as_str()),
                    passphrase_credential_id.as_deref(),
                ),
                AuthStep::KeyboardInteractive => ("keyboardInteractive", None, None, None),
            };
            for (reference, kind) in [(credential, "password"), (passphrase, "passphrase")] {
                if let Some(reference) = reference {
                    let ready: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM credentials WHERE id=? AND kind=? AND status='ready'",
                    )
                    .bind(reference)
                    .bind(kind)
                    .fetch_one(&mut *tx)
                    .await?;
                    if ready != 1 {
                        return Err(StorageError::Validation("认证引用不可用或类型不符"));
                    }
                }
            }
            sqlx::query("INSERT INTO profile_auth_steps(id,profile_id,sort_order,method,credential_id,key_file_id,passphrase_credential_id,step_json) VALUES(?,?,?,?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(&id).bind(index as i64).bind(method).bind(credential).bind(key_file).bind(passphrase).bind(serde_json::to_string(step)?).execute(&mut *tx).await?;
        }
        for tag in &draft.tags {
            sqlx::query("INSERT INTO tags(id,name) VALUES(?,?) ON CONFLICT(name) DO NOTHING")
                .bind(Uuid::new_v4().to_string())
                .bind(tag.trim())
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT OR IGNORE INTO profile_tags(profile_id,tag_id) SELECT ?,id FROM tags WHERE name=?").bind(&id).bind(tag.trim()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        self.get_host(&id).await
    }
    pub async fn delete_host(&self, id: &str, revision: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM connection_profiles WHERE id=? AND revision=?")
            .bind(id)
            .bind(revision)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() != 1 {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }
    pub async fn duplicate_host(&self, id: &str) -> Result<HostProfile> {
        let host = self.get_host(id).await?;
        let name = format!("{} 副本", host.name.chars().take(125).collect::<String>());
        self.save_host(HostDraft {
            id: None,
            revision: None,
            name,
            address: host.address,
            port: host.port,
            username: host.username,
            group_id: host.group_id,
            favorite: host.favorite,
            tags: host.tags,
            auth_plan: host.auth_plan,
        })
        .await
    }
}

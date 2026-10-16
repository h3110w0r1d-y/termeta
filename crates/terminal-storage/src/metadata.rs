use crate::*;
use async_trait::async_trait;
use sqlx::Row;
use terminal_core::{HostKey, HostTrustStore, SessionError, TrustStatus};
use uuid::Uuid;

impl SqliteStore {
    pub async fn list_known_hosts(&self) -> Result<Vec<TrustedHostKey>> {
        sqlx::query("SELECT * FROM known_hosts ORDER BY canonical_host,port")
            .fetch_all(self.pool()?)
            .await?
            .into_iter()
            .map(|r| {
                Ok(TrustedHostKey {
                    id: r.try_get("id")?,
                    canonical_host: r.try_get("canonical_host")?,
                    port: r.try_get::<i64, _>("port")? as u16,
                    key_algorithm: r.try_get("key_algorithm")?,
                    public_key: r.try_get("public_key")?,
                    fingerprint_sha256: r.try_get("fingerprint_sha256")?,
                    trusted_at: r.try_get("trusted_at")?,
                })
            })
            .collect()
    }
    pub async fn known_keys(&self, host: &str, port: u16) -> Result<Vec<TrustedHostKey>> {
        let canonical = canonical_host(host);
        Ok(self
            .list_known_hosts()
            .await?
            .into_iter()
            .filter(|k| k.canonical_host == canonical && k.port == port)
            .collect())
    }
    pub async fn trust_key(&self, key: TrustedHostKey) -> Result<()> {
        let mut tx = self.pool()?.begin_with("BEGIN IMMEDIATE").await?;
        let host = canonical_host(&key.canonical_host);
        let existing = sqlx::query(
            "SELECT public_key,key_algorithm FROM known_hosts WHERE canonical_host=? AND port=?",
        )
        .bind(&host)
        .bind(i64::from(key.port))
        .fetch_all(&mut *tx)
        .await?;
        if !existing.is_empty()
            && !existing.iter().any(|r| {
                r.get::<Vec<u8>, _>("public_key") == key.public_key
                    && r.get::<String, _>("key_algorithm") == key.key_algorithm
            })
        {
            return Err(StorageError::Validation(
                "主机密钥已变更，请先在信任管理中明确撤销旧记录",
            ));
        }
        sqlx::query("INSERT INTO known_hosts(id,canonical_host,port,key_algorithm,public_key,fingerprint_sha256,trusted_at,last_seen_at) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(canonical_host,port,key_algorithm,public_key) DO UPDATE SET last_seen_at=excluded.last_seen_at")
            .bind(key.id).bind(host).bind(i64::from(key.port)).bind(key.key_algorithm).bind(key.public_key).bind(key.fingerprint_sha256).bind(now()).bind(now()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn revoke_key(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM known_hosts WHERE id=?")
            .bind(id)
            .execute(self.pool()?)
            .await?;
        Ok(())
    }
    pub async fn get_setting(&self, key: &str) -> Result<Option<StoredSetting>> {
        let row = sqlx::query("SELECT value_json,value_version FROM settings WHERE key=?")
            .bind(key)
            .fetch_optional(self.pool()?)
            .await?;
        row.map(|r| {
            Ok(StoredSetting {
                value_version: r.try_get("value_version")?,
                value: serde_json::from_str(r.try_get("value_json")?)?,
            })
        })
        .transpose()
    }
    pub async fn set_setting(
        &self,
        key: &str,
        version: i64,
        value: &serde_json::Value,
    ) -> Result<()> {
        if key.len() > 128 || version < 1 {
            return Err(StorageError::Validation("设置键或版本无效"));
        }
        let encoded = serde_json::to_string(value)?;
        if encoded.len() > 1024 * 1024 {
            return Err(StorageError::Validation("设置数据过大"));
        }
        sqlx::query("INSERT INTO settings(key,value_json,value_version,updated_at) VALUES(?,?,?,?) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,value_version=excluded.value_version,updated_at=excluded.updated_at")
            .bind(key).bind(encoded).bind(version).bind(now()).execute(self.pool()?).await?;
        Ok(())
    }
    pub async fn register_key_file(&self, path: &str, display_name: &str) -> Result<KeyFile> {
        validate_name(display_name)?;
        let selected = std::path::PathBuf::from(path);
        let fingerprint = tokio::task::spawn_blocking(move || {
            let (_secret, fingerprint) = crate::read_key_file(&selected, None)?;
            Ok::<_, StorageError>(fingerprint)
        })
        .await
        .map_err(|_| StorageError::Validation("私钥校验工作器不可用"))??;
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO key_files(id,path,display_name,content_fingerprint,created_at,updated_at) VALUES(?,?,?,?,?,?) ON CONFLICT(path) DO UPDATE SET display_name=excluded.display_name,content_fingerprint=excluded.content_fingerprint,updated_at=excluded.updated_at")
            .bind(&id).bind(path).bind(display_name).bind(fingerprint).bind(now()).bind(now()).execute(self.pool()?).await?;
        let row = sqlx::query("SELECT * FROM key_files WHERE path=?")
            .bind(path)
            .fetch_one(self.pool()?)
            .await?;
        Ok(KeyFile {
            id: row.try_get("id")?,
            path: row.try_get("path")?,
            display_name: row.try_get("display_name")?,
            content_fingerprint: row.try_get("content_fingerprint")?,
        })
    }
    pub async fn get_key_file(&self, id: &str) -> Result<KeyFile> {
        let row = sqlx::query("SELECT * FROM key_files WHERE id=?")
            .bind(id)
            .fetch_optional(self.pool()?)
            .await?
            .ok_or(StorageError::NotFound)?;
        Ok(KeyFile {
            id: row.try_get("id")?,
            path: row.try_get("path")?,
            display_name: row.try_get("display_name")?,
            content_fingerprint: row.try_get("content_fingerprint")?,
        })
    }
    pub async fn start_history(
        &self,
        profile_id: Option<&str>,
        name: &str,
        endpoint: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO session_history(id,profile_id,display_name_snapshot,endpoint_snapshot,started_at) VALUES(?,?,?,?,?)").bind(&id).bind(profile_id).bind(name).bind(endpoint).bind(now()).execute(self.pool()?).await?;
        Ok(id)
    }
    pub async fn end_history(
        &self,
        id: &str,
        reason: &str,
        error_code: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE session_history SET ended_at=?,end_reason=?,error_code=? WHERE id=? AND ended_at IS NULL").bind(now()).bind(reason).bind(error_code).bind(id).execute(self.pool()?).await?;
        Ok(())
    }
    pub async fn list_history(&self) -> Result<Vec<HistoryEntry>> {
        sqlx::query("SELECT * FROM session_history ORDER BY started_at DESC LIMIT 1000")
            .fetch_all(self.pool()?)
            .await?
            .into_iter()
            .map(|r| {
                Ok(HistoryEntry {
                    id: r.try_get("id")?,
                    profile_id: r.try_get("profile_id")?,
                    display_name: r.try_get("display_name_snapshot")?,
                    endpoint: r.try_get("endpoint_snapshot")?,
                    started_at: r.try_get("started_at")?,
                    ended_at: r.try_get("ended_at")?,
                    end_reason: r.try_get("end_reason")?,
                    error_code: r.try_get("error_code")?,
                })
            })
            .collect()
    }
    pub async fn clear_history(&self) -> Result<()> {
        sqlx::query("DELETE FROM session_history")
            .execute(self.pool()?)
            .await?;
        Ok(())
    }
    pub async fn prune_history(&self, days: u16) -> Result<()> {
        sqlx::query("DELETE FROM session_history WHERE started_at<?")
            .bind(now() - i64::from(days) * 86_400_000)
            .execute(self.pool()?)
            .await?;
        Ok(())
    }
}
fn trust_error(_: StorageError) -> SessionError {
    SessionError::new(
        "TRUST_STORE_FAILED",
        "hostKey",
        "主机信任仓库不可用或密钥已变更",
    )
}
#[async_trait]
impl HostTrustStore for SqliteStore {
    async fn verify(&self, key: &HostKey) -> terminal_core::Result<TrustStatus> {
        let keys = self
            .known_keys(&key.endpoint.canonical_host, key.endpoint.port)
            .await
            .map_err(trust_error)?;
        Ok(if keys.is_empty() {
            TrustStatus::Unknown
        } else if keys
            .iter()
            .any(|k| k.public_key == key.public_key && k.key_algorithm == key.algorithm)
        {
            TrustStatus::Trusted
        } else {
            TrustStatus::Changed
        })
    }
    async fn trust(&self, key: &HostKey) -> terminal_core::Result<()> {
        self.trust_key(TrustedHostKey {
            id: Uuid::new_v4().to_string(),
            canonical_host: key.endpoint.canonical_host.clone(),
            port: key.endpoint.port,
            key_algorithm: key.algorithm.clone(),
            public_key: key.public_key.clone(),
            fingerprint_sha256: key.fingerprint.clone(),
            trusted_at: now(),
        })
        .await
        .map_err(trust_error)
    }
}

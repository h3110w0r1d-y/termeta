use crate::*;
use sqlx::Row;
use terminal_core::{AuthStep, SshConfig, SshStartup, TelnetConfig};
use uuid::Uuid;

/// 只兼容缺失字段；未知类型、空值和无效命令不能退回 Shell。
fn ssh_startup(config: &serde_json::Value) -> Result<SshStartup> {
    let startup: SshStartup = match config.get("startup") {
        Some(value) => serde_json::from_value(value.clone())?,
        None => SshStartup::default(),
    };
    startup
        .validate()
        .map_err(|_| StorageError::Validation("SSH 启动配置无效"))?;
    Ok(startup)
}
impl SqliteStore {
    pub async fn list_hosts(&self, search: &str) -> Result<Vec<HostProfile>> {
        let rows = sqlx::query("SELECT p.*,s.address,s.port,s.username FROM connection_profiles p LEFT JOIN ssh_profiles s ON s.profile_id=p.id WHERE p.kind IN ('ssh','serial','telnet','workspace') ORDER BY p.favorite DESC,p.name COLLATE NOCASE")
            .fetch_all(&*self.pool().await?).await?;
        let mut profiles = Vec::with_capacity(rows.len());
        let search = search.to_lowercase();
        for row in rows {
            let id: String = row.try_get("id")?;
            let tags = sqlx::query_scalar::<_, String>("SELECT t.name FROM tags t JOIN profile_tags p ON p.tag_id=t.id WHERE p.profile_id=? ORDER BY t.name")
                .bind(&id).fetch_all(&*self.pool().await?).await?;
            let auth = sqlx::query("SELECT a.method,a.credential_id,c.owner_profile_id FROM profile_auth_steps a LEFT JOIN credentials c ON c.id=a.credential_id WHERE a.profile_id=? ORDER BY a.sort_order")
                .bind(&id).fetch_all(&*self.pool().await?).await?;
            let mut has_password = false;
            let mut auth_plan = Vec::with_capacity(auth.len());
            for step in auth {
                let method: String = step.try_get("method")?;
                let credential_id: Option<String> = step.try_get("credential_id")?;
                let owner: Option<String> = step.try_get("owner_profile_id")?;
                auth_plan.push(match method.as_str() {
                    "password" => {
                        if owner.is_some() {
                            has_password = true;
                            AuthStep::Password {
                                credential_id: None,
                            }
                        } else {
                            AuthStep::Password { credential_id }
                        }
                    }
                    "privateKey" => AuthStep::PrivateKey {
                        credential_id: credential_id
                            .ok_or(StorageError::Validation("私钥凭据引用缺失"))?,
                    },
                    _ => return Err(StorageError::Validation("认证类型无效")),
                });
            }
            let config: serde_json::Value = serde_json::from_str(row.try_get("config_json")?)?;
            let kind: String = row.try_get("kind")?;
            let serial_config = if kind == "serial" {
                Some(serde_json::from_value(config["serialConfig"].clone())?)
            } else {
                None
            };
            let telnet = if kind == "telnet" {
                let target: TelnetConfig = serde_json::from_value(config["telnetConfig"].clone())?;
                target
                    .validate()
                    .map_err(|_| StorageError::Validation("Telnet 配置无效"))?;
                Some(target)
            } else {
                None
            };
            let host = HostProfile {
                startup: if kind == "ssh" {
                    ssh_startup(&config)?
                } else {
                    SshStartup::default()
                },
                workspace: if kind == "workspace" {
                    Some(serde_json::from_value(config["workspace"].clone())?)
                } else {
                    None
                },
                kind,
                serial_config,
                icon: config["icon"].as_str().unwrap_or("ssh").to_owned(),
                system_detection_attempted: config["systemDetectionAttempted"]
                    .as_bool()
                    .unwrap_or(false),
                id,
                revision: row.try_get("revision")?,
                name: row.try_get("name")?,
                address: if let Some(target) = &telnet {
                    target.address.clone()
                } else {
                    row.try_get::<Option<String>, _>("address")?
                        .unwrap_or_default()
                },
                port: if let Some(target) = &telnet {
                    target.port
                } else {
                    row.try_get::<Option<i64>, _>("port")?.unwrap_or_default() as u16
                },
                username: row
                    .try_get::<Option<String>, _>("username")?
                    .unwrap_or_default(),
                group_id: row.try_get("group_id")?,
                favorite: row.try_get("favorite")?,
                tags,
                auth_plan,
                has_password,
            };
            if search.is_empty()
                || [
                    &host.name,
                    &host.address,
                    &host.username,
                    host.serial_config
                        .as_ref()
                        .map(|config| &config.path)
                        .unwrap_or(&host.address),
                ]
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
    /// 连接配置只供后端使用，主机专属引用不得经公共主机 DTO 返回。
    pub async fn get_host_connection(&self, id: &str) -> Result<SshConfig> {
        let mut tx = self.pool().await?.begin().await?;
        let profile = sqlx::query("SELECT kind,config_json FROM connection_profiles WHERE id=?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound)?;
        if profile.try_get::<String, _>("kind")? != "ssh" {
            return Err(StorageError::Validation(
                "只有 SSH 主机支持 SSH 与 SFTP 连接",
            ));
        }
        let row = sqlx::query("SELECT address,port,username FROM ssh_profiles WHERE profile_id=?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound)?;
        let rows = sqlx::query("SELECT a.method,a.credential_id,c.kind,c.status,c.owner_profile_id FROM profile_auth_steps a LEFT JOIN credentials c ON c.id=a.credential_id WHERE a.profile_id=? ORDER BY a.sort_order")
            .bind(id).fetch_all(&mut *tx).await?;
        let mut auth_plan = Vec::with_capacity(rows.len());
        for step in rows {
            let method: String = step.try_get("method")?;
            let credential_id: Option<String> = step.try_get("credential_id")?;
            if credential_id.is_some() {
                let owner: Option<String> = step.try_get("owner_profile_id")?;
                let kind: Option<String> = step.try_get("kind")?;
                let status: Option<String> = step.try_get("status")?;
                if kind.as_deref() != Some(method.as_str())
                    || status.as_deref() != Some("ready")
                    || owner.as_deref().is_some_and(|owner| owner != id)
                {
                    return Err(StorageError::Validation("认证引用不可用或归属不符"));
                }
            }
            auth_plan.push(match method.as_str() {
                "password" => AuthStep::Password { credential_id },
                "privateKey" => AuthStep::PrivateKey {
                    credential_id: credential_id
                        .ok_or(StorageError::Validation("私钥凭据引用缺失"))?,
                },
                _ => return Err(StorageError::Validation("认证类型无效")),
            });
        }
        let config = SshConfig {
            startup: ssh_startup(&serde_json::from_str(profile.try_get("config_json")?)?)?,
            config_version: 1,
            system_profile_id: None,
            address: row.try_get("address")?,
            port: row.try_get::<i64, _>("port")? as u16,
            username: row.try_get("username")?,
            auth_plan,
        };
        tx.commit().await?;
        Ok(config)
    }
    /// 元数据修改保留当前专属密码；显式编辑密码使用下面的替换接口。
    pub async fn save_host(&self, draft: HostDraft) -> Result<HostProfile> {
        self.save_host_inner(draft, None, true).await
    }
    pub async fn save_host_with_password_ref(
        &self,
        draft: HostDraft,
        password_ref: Option<&str>,
    ) -> Result<HostProfile> {
        self.save_host_inner(draft, password_ref, false).await
    }
    async fn save_host_inner(
        &self,
        draft: HostDraft,
        password_ref: Option<&str>,
        preserve_password: bool,
    ) -> Result<HostProfile> {
        if draft.kind == "workspace" {
            if password_ref.is_some() {
                return Err(StorageError::Validation("工作区不支持凭据"));
            }
            return self.update_workspace_metadata(draft).await;
        }
        if draft.kind != "ssh" && draft.startup != SshStartup::Shell {
            return Err(StorageError::Validation("只有 SSH 主机支持启动命令"));
        }
        validate_name(&draft.name)?;
        if !terminal_core::HOST_SYSTEM_ICONS.contains(&draft.icon.as_str()) {
            return Err(StorageError::Validation("主机图标无效"));
        }
        match draft.kind.as_str() {
            "ssh" => {
                if draft.serial_config.is_some() {
                    return Err(StorageError::Validation("SSH 主机不能包含串口配置"));
                }
                SshConfig {
                    startup: draft.startup.clone(),
                    config_version: 1,
                    system_profile_id: None,
                    address: draft.address.clone(),
                    port: draft.port,
                    username: draft.username.clone(),
                    auth_plan: draft.auth_plan.clone(),
                }
                .validate()
                .map_err(|_| StorageError::Validation("SSH 配置无效"))?;
            }
            "telnet" => {
                if draft.serial_config.is_some()
                    || !draft.username.is_empty()
                    || !draft.auth_plan.is_empty()
                    || password_ref.is_some()
                {
                    return Err(StorageError::Validation(
                        "Telnet 主机不能包含 SSH 凭据或串口配置",
                    ));
                }
                TelnetConfig {
                    config_version: 1,
                    address: draft.address.clone(),
                    port: draft.port,
                }
                .validate()
                .map_err(|_| StorageError::Validation("Telnet 配置无效"))?;
            }
            "serial" => {
                if !draft.address.is_empty()
                    || draft.port != 0
                    || !draft.username.is_empty()
                    || !draft.auth_plan.is_empty()
                    || password_ref.is_some()
                {
                    return Err(StorageError::Validation("串口主机不能包含 SSH 配置或凭据"));
                }
                // 保存只校验参数语法，不枚举、打开或要求设备当前存在。
                draft
                    .serial_config
                    .as_ref()
                    .ok_or(StorageError::Validation("串口配置缺失"))?
                    .validate()
                    .map_err(|_| StorageError::Validation("串口配置无效"))?;
            }
            _ => return Err(StorageError::Validation("主机类型无效")),
        }
        if draft.tags.len() > 32
            || draft.tags.iter().any(|t| {
                t.trim().is_empty() || t.chars().count() > 64 || t.chars().any(char::is_control)
            })
        {
            return Err(StorageError::Validation("标签数量或长度无效"));
        }
        let direct_password = matches!(
            draft.auth_plan.as_slice(),
            [AuthStep::Password {
                credential_id: None
            }]
        );
        if password_ref.is_some() && !direct_password {
            return Err(StorageError::Validation("主机专属密码仅适用于直接密码认证"));
        }
        let mut tx = self.pool().await?.begin_with("BEGIN IMMEDIATE").await?;
        let scoped_owner = if let Some(reference) = password_ref {
            Some(sqlx::query_scalar::<_, Option<String>>("SELECT owner_profile_id FROM credentials WHERE id=? AND kind='password' AND status='ready'")
                .bind(reference).fetch_optional(&mut *tx).await?.flatten().ok_or(StorageError::Validation("主机密码引用不可用"))?)
        } else {
            None
        };
        let id = draft
            .id
            .clone()
            .or_else(|| scoped_owner.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        ensure_unique_profile_name(&mut tx, draft.name.trim(), Some(&id)).await?;
        if scoped_owner.as_deref().is_some_and(|owner| owner != id) {
            return Err(StorageError::Validation("主机密码归属不符"));
        }
        let preserved_ref = if preserve_password && direct_password && draft.id.is_some() {
            sqlx::query_scalar::<_, String>("SELECT c.id FROM credentials c JOIN profile_auth_steps a ON a.credential_id=c.id WHERE a.profile_id=? AND a.method='password' AND c.owner_profile_id=? AND c.status='ready' LIMIT 1")
                .bind(&id).bind(&id).fetch_optional(&mut *tx).await?
        } else {
            None
        };
        let password_ref = password_ref.or(preserved_ref.as_deref());
        if draft.id.is_some() {
            let kind: String =
                sqlx::query_scalar("SELECT kind FROM connection_profiles WHERE id=?")
                    .bind(&id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or(StorageError::NotFound)?;
            if kind != draft.kind {
                return Err(StorageError::Validation("不能更改已保存主机的连接类型"));
            }
            let revision = draft.revision.ok_or(StorageError::Conflict)?;
            let changed = sqlx::query("UPDATE connection_profiles SET name=?,group_id=?,favorite=?,revision=revision+1,updated_at=? WHERE id=? AND revision=?")
                .bind(draft.name.trim()).bind(&draft.group_id).bind(draft.favorite).bind(now()).bind(&id).bind(revision).execute(&mut *tx).await?;
            if changed.rows_affected() != 1 {
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
            sqlx::query("INSERT INTO connection_profiles(id,kind,name,group_id,favorite,created_at,updated_at) VALUES(?,?,?,?,?,?,?)")
                .bind(&id).bind(&draft.kind).bind(draft.name.trim()).bind(&draft.group_id).bind(draft.favorite).bind(now()).bind(now()).execute(&mut *tx).await?;
        }
        // 只合并可编辑字段；检测标记和令牌不能由主机表单回写。
        sqlx::query("UPDATE connection_profiles SET config_json=json_set(CASE WHEN coalesce(json_extract(config_json,'$.icon'),'ssh')<>? OR EXISTS(SELECT 1 FROM ssh_profiles WHERE profile_id=? AND (address<>? OR port<>? OR username<>?)) THEN json_remove(config_json,'$.systemDetectionToken') ELSE config_json END,'$.icon',?) WHERE id=?")
            .bind(&draft.icon).bind(&id).bind(&draft.address).bind(i64::from(draft.port)).bind(&draft.username)
            .bind(&draft.icon).bind(&id).execute(&mut *tx).await?;
        if draft.kind == "ssh" {
            sqlx::query("UPDATE connection_profiles SET config_json=json_set(config_json,'$.startup',json(?)) WHERE id=?")
                .bind(serde_json::to_string(&draft.startup)?).bind(&id).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO ssh_profiles(profile_id,address,canonical_host,port,username) VALUES(?,?,?,?,?) ON CONFLICT(profile_id) DO UPDATE SET address=excluded.address,canonical_host=excluded.canonical_host,port=excluded.port,username=excluded.username")
                .bind(&id).bind(&draft.address).bind(canonical_host(&draft.address)).bind(i64::from(draft.port)).bind(&draft.username).execute(&mut *tx).await?;
        } else if draft.kind == "telnet" {
            let config = serde_json::to_string(&TelnetConfig {
                config_version: 1,
                address: draft.address.clone(),
                port: draft.port,
            })?;
            sqlx::query("UPDATE connection_profiles SET config_json=json_set(config_json,'$.telnetConfig',json(?)) WHERE id=?")
                .bind(config).bind(&id).execute(&mut *tx).await?;
        } else {
            let serial_config = serde_json::to_string(&draft.serial_config)?;
            sqlx::query("UPDATE connection_profiles SET config_json=json_set(config_json,'$.serialConfig',json(?)) WHERE id=?")
                .bind(serial_config).bind(&id).execute(&mut *tx).await?;
        }
        for (index, step) in draft.auth_plan.iter().enumerate() {
            let (method, public_reference) = match step {
                AuthStep::Password { credential_id } => ("password", credential_id.as_deref()),
                AuthStep::PrivateKey { credential_id } => {
                    ("privateKey", Some(credential_id.as_str()))
                }
            };
            if let Some(reference) = public_reference {
                let ready: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credentials WHERE id=? AND kind=? AND status='ready' AND owner_profile_id IS NULL")
                    .bind(reference).bind(method).fetch_one(&mut *tx).await?;
                if ready != 1 {
                    return Err(StorageError::Validation("公共认证引用不可用或类型不符"));
                }
            }
            let reference = public_reference.or(if method == "password" {
                password_ref
            } else {
                None
            });
            sqlx::query("INSERT INTO profile_auth_steps(id,profile_id,sort_order,method,credential_id) VALUES(?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string()).bind(&id).bind(index as i64).bind(method).bind(reference).execute(&mut *tx).await?;
        }
        // 主机与凭据关系变更一起提交，系统凭据删除失败可由协调器恢复。
        sqlx::query("UPDATE credentials SET status='deleting',updated_at=? WHERE owner_profile_id=? AND (? IS NULL OR id<>?)")
            .bind(now()).bind(&id).bind(password_ref).bind(password_ref).execute(&mut *tx).await?;
        for tag in &draft.tags {
            sqlx::query("INSERT INTO tags(id,name) VALUES(?,?) ON CONFLICT(name) DO NOTHING")
                .bind(Uuid::new_v4().to_string())
                .bind(tag.trim())
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT OR IGNORE INTO profile_tags(profile_id,tag_id) SELECT ?,id FROM tags WHERE name=?")
                .bind(&id).bind(tag.trim()).execute(&mut *tx).await?;
        }
        // 在事务内取得本次保存的准确结果，提交后不再执行可失败的重读。
        let revision: i64 =
            sqlx::query_scalar("SELECT revision FROM connection_profiles WHERE id=?")
                .bind(&id)
                .fetch_one(&mut *tx)
                .await?;
        let tags = sqlx::query_scalar::<_, String>("SELECT t.name FROM tags t JOIN profile_tags p ON p.tag_id=t.id WHERE p.profile_id=? ORDER BY t.name")
            .bind(&id).fetch_all(&mut *tx).await?;
        let config_json: String =
            sqlx::query_scalar("SELECT config_json FROM connection_profiles WHERE id=?")
                .bind(&id)
                .fetch_one(&mut *tx)
                .await?;
        let metadata: serde_json::Value = serde_json::from_str(&config_json)?;
        let saved = HostProfile {
            startup: draft.startup,
            workspace: None,
            kind: draft.kind,
            serial_config: draft.serial_config,
            icon: draft.icon,
            system_detection_attempted: metadata["systemDetectionAttempted"]
                .as_bool()
                .unwrap_or(false),
            id,
            revision,
            name: draft.name.trim().to_owned(),
            address: draft.address,
            port: draft.port,
            username: draft.username,
            group_id: draft.group_id,
            favorite: draft.favorite,
            tags,
            // draft 只允许公共引用或空密码引用，专属定位符不进入返回值。
            auth_plan: draft.auth_plan,
            has_password: password_ref.is_some(),
        };
        tx.commit().await?;
        Ok(saved)
    }
    pub async fn delete_host(&self, id: &str, revision: i64) -> Result<()> {
        let mut tx = self.pool().await?.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query(
            "UPDATE credentials SET status='deleting',updated_at=? WHERE owner_profile_id=?",
        )
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        let result = sqlx::query("DELETE FROM connection_profiles WHERE id=? AND revision=?")
            .bind(id)
            .bind(revision)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() != 1 {
            return Err(StorageError::Conflict);
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn duplicate_host(&self, id: &str) -> Result<HostProfile> {
        let host = self.get_host(id).await?;
        self.save_host(HostDraft {
            startup: host.startup,
            kind: host.kind,
            serial_config: host.serial_config,
            icon: host.icon,
            id: None,
            revision: None,
            name: format!("{} 副本", host.name.chars().take(125).collect::<String>()),
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

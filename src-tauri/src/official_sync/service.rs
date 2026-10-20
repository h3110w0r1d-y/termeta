//! 启动账号密码门禁与轻量状态查询，不承接终端资源。
use super::{network::head, runtime::Status, sync::synchronize};
use crate::{
    account::{
        failure,
        network::Api,
        storage::{self, SavedAccount},
    },
    state::{AppState, storage_error},
};
use terminal_core::Result;
use terminal_security::backup;
use zeroize::Zeroizing;
pub(super) async fn account(state: &AppState, api: &Api) -> Result<Option<SavedAccount>> {
    storage::load(state, &api.origin).await
}
pub(super) fn needs_password(state: &AppState, has_data: bool, error: Option<String>) -> Status {
    state.official.set(Status {
        phase: "needsPassword",
        cloud_has_data: has_data,
        error,
        last_synced_at: state.official.value().last_synced_at,
    })
}
pub async fn status(state: &AppState) -> Result<Status> {
    let api = Api::new()?;
    // 状态查询等待当前同步完成，不能把启动时的旧摘要当作门禁结论返回。
    let _operation = state.official.operation.lock().await;
    let Some(account) = account(state, &api).await? else {
        return Ok(state.official.set(Status::default()));
    };
    if state.official.ready() {
        return Ok(state.official.value());
    }
    match head(&api, &account).await {
        Ok(head) => {
            let staged = state.account.pending.lock().unwrap().is_some();
            let fresh = state
                .official
                .requires_password
                .load(std::sync::atomic::Ordering::Acquire);
            if !staged && !fresh && account.sync_confirmed && state.vault.is_unlocked() {
                match synchronize(state, &api, &account, Some(head)).await {
                    Ok(value) => return Ok(value),
                    Err(error) => {
                        return Ok(needs_password(state, true, Some(error.message)));
                    }
                }
            }
            Ok(needs_password(
                state,
                head.encrypted_content.is_some(),
                None,
            ))
        }
        Err(error) => {
            if account.sync_confirmed
                && !state
                    .official
                    .requires_password
                    .load(std::sync::atomic::Ordering::Acquire)
                && state.vault.is_unlocked()
                && state
                    .store
                    .official_baseline(&api.origin, &account.user.id)
                    .await
                    .map_err(storage_error)?
                    .is_some()
            {
                return Ok(state.official.set(Status {
                    phase: "ready",
                    cloud_has_data: true,
                    last_synced_at: None,
                    error: Some(error.message),
                }));
            }
            Ok(needs_password(state, false, Some(error.message)))
        }
    }
}

/// 密码只用于本机认证和解密；认证失败前不创建库、不提交合并、不上传。
pub async fn complete(state: &AppState, password: Zeroizing<String>) -> Result<Status> {
    let _operation = state
        .official
        .operation
        .try_lock()
        .map_err(|_| failure("SYNC_BUSY", "同步正在进行，请稍后重试"))?;
    let _initialization = state.backup.initialization.lock().await;
    let api = Api::new()?;
    let mut account = account(state, &api)
        .await?
        .ok_or_else(|| failure("ACCOUNT_UNAUTHORIZED", "请先登录官方账号"))?;
    let cloud = head(&api, &account).await?;
    if let Some(package) = &cloud.encrypted_content {
        let package = package.clone();
        let (material, _opened) = tokio::task::spawn_blocking(move || {
            let salt = backup::package_salt(&package)?;
            let material = terminal_security::derive_vault_key(&password, salt)?;
            let opened = backup::open_with_key(&material.key, &package)?;
            Ok::<_, terminal_security::SecurityError>((material, opened))
        })
        .await
        .map_err(|_| failure("SYNC_INTERRUPTED", "云端解密任务中断"))?
        .map_err(crate::commands::security_error)?;
        // 已有锁定库必须先独立解锁，不能用已认证的云包覆盖本机数据。
        if !state.vault.is_unlocked() && state.store.path().exists() {
            return Err(failure(
                "SYNC_LOCAL_LOCKED",
                "请先解锁本机保险库，再验证云端密码",
            ));
        }
        state
            .vault
            .adopt_master_key(material)
            .await
            .map_err(crate::commands::security_error)?;
    } else if state.vault.is_unlocked() {
        state
            .vault
            .verify_password(password)
            .await
            .map_err(crate::commands::security_error)?;
    } else {
        let setup = state
            .vault
            .status()
            .await
            .map_err(crate::commands::security_error)?
            .mode
            == "setup";
        state
            .vault
            .unlock_with_password(password, setup)
            .await
            .map_err(crate::commands::security_error)?;
    }
    account.sync_confirmed = true;
    storage::save(state, Some(&account)).await?;
    // 密码认证成功后即允许离线继续；上传失败保留错误并由后台重试。
    state
        .official
        .requires_password
        .store(false, std::sync::atomic::Ordering::Release);
    state.official.set(Status {
        phase: "ready",
        cloud_has_data: cloud.encrypted_content.is_some(),
        last_synced_at: None,
        error: None,
    });
    match synchronize(state, &api, &account, Some(cloud)).await {
        Ok(status) => Ok(status),
        Err(error) => {
            let mut value = state.official.value();
            value.error = Some(error.message);
            Ok(state.official.set(value))
        }
    }
}

pub async fn sync(state: &AppState) -> Result<Status> {
    let _operation = state
        .official
        .operation
        .try_lock()
        .map_err(|_| failure("SYNC_BUSY", "同步正在进行，请稍后重试"))?;
    if !state.official.ready() {
        return Ok(state.official.value());
    }
    let api = Api::new()?;
    let Some(account) = account(state, &api).await? else {
        return Ok(state.official.set(Status::default()));
    };
    match synchronize(state, &api, &account, None).await {
        Ok(value) => Ok(value),
        Err(error) => {
            let mut value = state.official.value();
            value.error = Some(error.message.clone());
            state.official.set(value);
            Err(error)
        }
    }
}

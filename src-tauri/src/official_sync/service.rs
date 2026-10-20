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
use terminal_core::{Result, SessionError};
use terminal_security::backup;
use zeroize::Zeroizing;
pub(super) async fn account(state: &AppState, api: &Api) -> Result<Option<SavedAccount>> {
    storage::load(state, &api.origin)
        .await
        .inspect_err(|error| {
            failed(state, error, false, state.official.value().cloud_has_data);
        })
}
pub(super) fn needs_password(state: &AppState, has_data: bool, error: Option<String>) -> Status {
    state
        .official
        .requires_password
        .store(true, std::sync::atomic::Ordering::Release);
    state.official.set(Status {
        phase: "needsPassword",
        cloud_has_data: has_data,
        error,
        last_synced_at: state.official.value().last_synced_at,
    })
}

/// 只有传输失败、服务暂不可用和有界 CAS 冲突允许沿用已验证的本机使用资格。
/// 包损坏、密钥变化、撤销与存储错误必须关闭门禁，不能按“稍后重试”泛化放行。
fn retryable(code: &str) -> bool {
    matches!(
        code,
        "ACCOUNT_NETWORK"
            | "ACCOUNT_UNAVAILABLE"
            | "ACCOUNT_RATE_LIMITED"
            | "ACCOUNT_CONFLICT"
            | "SYNC_CONFLICT"
    )
}

fn failure_status(
    previous: Status,
    error: &SessionError,
    can_resume: bool,
    has_data: bool,
) -> Status {
    Status {
        phase: if can_resume && retryable(&error.code) {
            "ready"
        } else {
            "needsPassword"
        },
        cloud_has_data: has_data,
        last_synced_at: previous.last_synced_at,
        error: Some(error.message.clone()),
    }
}

pub(super) fn failed(
    state: &AppState,
    error: &SessionError,
    can_resume: bool,
    has_data: bool,
) -> Status {
    let status = failure_status(state.official.value(), error, can_resume, has_data);
    if status.phase == "needsPassword" {
        state
            .official
            .requires_password
            .store(true, std::sync::atomic::Ordering::Release);
    }
    state.official.set(status)
}

async fn can_resume(state: &AppState, api: &Api, account: &SavedAccount) -> Result<bool> {
    if !account.sync_confirmed
        || state.account.pending.lock().unwrap().is_some()
        || state
            .official
            .requires_password
            .load(std::sync::atomic::Ordering::Acquire)
        || !state.vault.is_unlocked()
    {
        return Ok(false);
    }
    Ok(state
        .store
        .official_baseline(&api.origin, &account.user.id)
        .await
        .map_err(storage_error)?
        .is_some())
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
            let has_data = head.encrypted_content.is_some();
            let staged = state.account.pending.lock().unwrap().is_some();
            let fresh = state
                .official
                .requires_password
                .load(std::sync::atomic::Ordering::Acquire);
            if !staged && !fresh && account.sync_confirmed && state.vault.is_unlocked() {
                match synchronize(state, &api, &account, Some(head)).await {
                    Ok(value) => return Ok(value),
                    Err(error) => {
                        let resume =
                            retryable(&error.code) && can_resume(state, &api, &account).await?;
                        return Ok(failed(state, &error, resume, has_data));
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
            let resume = retryable(&error.code) && can_resume(state, &api, &account).await?;
            Ok(failed(state, &error, resume, resume))
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
    let _mode = crate::account::service::begin_mode_change(state).await?;
    let _initialization = state.backup.initialization.lock().await;
    let api = Api::new()?;
    let mut account = account(state, &api)
        .await?
        .ok_or_else(|| failure("ACCOUNT_UNAUTHORIZED", "请先登录官方账号"))?;
    let cloud = match head(&api, &account).await {
        Ok(cloud) => cloud,
        Err(error) => {
            failed(
                state,
                &error,
                state.official.ready(),
                state.official.value().cloud_has_data,
            );
            return Err(error);
        }
    };
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
        Err(error) => Ok(failed(
            state,
            &error,
            true,
            state.official.value().cloud_has_data,
        )),
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
            failed(state, &error, true, state.official.value().cloud_has_data);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision(code: &str, recoverable_local: bool) -> Status {
        failure_status(
            Status {
                phase: "ready",
                cloud_has_data: true,
                last_synced_at: Some("上次同步时间".into()),
                error: None,
            },
            &failure(code, "同步诊断原文"),
            recoverable_local,
            true,
        )
    }

    #[test]
    fn get_put_timeout_and_cas_exhaustion_keep_only_verified_local_access() {
        for code in [
            "ACCOUNT_NETWORK",
            "ACCOUNT_UNAVAILABLE",
            "ACCOUNT_RATE_LIMITED",
            "ACCOUNT_CONFLICT",
            "SYNC_CONFLICT",
        ] {
            let status = decision(code, true);
            assert_eq!(status.phase, "ready");
            assert_eq!(status.error.as_deref(), Some("同步诊断原文"));
            assert_eq!(status.last_synced_at.as_deref(), Some("上次同步时间"));
            // 新账号、锁定库、无基线或要求重新验证时都没有可恢复资格。
            assert_eq!(decision(code, false).phase, "needsPassword");
        }
    }

    #[test]
    fn revoked_corrupt_key_changed_storage_and_unknown_failures_never_reuse_ready() {
        for code in [
            "ACCOUNT_UNAUTHORIZED",
            "ACCOUNT_FORBIDDEN",
            "ACCOUNT_RESPONSE",
            "SYNC_RESPONSE",
            "SYNC_PASSWORD_CHANGED",
            "CREDENTIAL_UNAVAILABLE",
            "STORAGE_UNAVAILABLE",
            "SYNC_INTERRUPTED",
            "UNKNOWN_ERROR",
        ] {
            assert_eq!(decision(code, true).phase, "needsPassword");
        }
    }
}

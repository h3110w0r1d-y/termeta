//! 整包同步使用已观测云基线与CAS，冲突后重新读取并有限重试。
use super::{
    envelope,
    network::{self, Head, head},
    runtime::Status,
    service::needs_password,
};
use crate::{
    account::{failure, network::Api, storage::SavedAccount},
    state::{AppState, storage_error},
};
use terminal_core::Result;
use terminal_security::backup;
use zeroize::Zeroizing;
pub(super) async fn synchronize(
    state: &AppState,
    api: &Api,
    account: &SavedAccount,
    initial: Option<Head>,
) -> Result<Status> {
    let material = state
        .vault
        .master_key()
        .map_err(crate::commands::security_error)?;
    let mut cloud = if let Some(value) = initial {
        value
    } else {
        head(api, account).await?
    };
    for _ in 0..3 {
        let baseline = state
            .store
            .official_baseline(&api.origin, &account.user.id)
            .await
            .map_err(storage_error)?;
        let mut remote_database = None;
        let database = if let Some(package) = cloud.encrypted_content.as_ref() {
            if backup::package_salt(package).map_err(crate::commands::security_error)?
                != material.salt
            {
                state
                    .official
                    .requires_password
                    .store(true, std::sync::atomic::Ordering::Release);
                needs_password(
                    state,
                    true,
                    Some("云端保险库密码已变化，请重新输入密码".into()),
                );
                return Err(failure(
                    "SYNC_PASSWORD_CHANGED",
                    "云端保险库密码已变化，请重新输入密码",
                ));
            }
            let package = package.clone();
            let key = material.key.clone();
            let remote = tokio::task::spawn_blocking(move || backup::open_with_key(&key, &package))
                .await
                .map_err(|_| failure("SYNC_INTERRUPTED", "同步解密中断"))?
                .map_err(crate::commands::security_error)?;
            let before = state
                .vault
                .export_backup_database()
                .await
                .map_err(crate::commands::security_error)?;
            let merged = state
                .vault
                .merge_official_database(
                    remote.database.clone(),
                    baseline,
                    Some(terminal_storage::official_sync::OfficialSyncObservation {
                        origin: &api.origin,
                        account_id: &account.user.id,
                        revision: cloud.revision,
                    }),
                )
                .await
                .map_err(crate::commands::security_error)?;
            if !equivalent(before, merged.clone()).await? {
                state
                    .official
                    .changed
                    .store(true, std::sync::atomic::Ordering::Release);
            }
            remote_database = Some(remote.database);
            merged
        } else {
            state
                .vault
                .export_backup_database()
                .await
                .map_err(crate::commands::security_error)?
        };
        if let Some(remote) = remote_database {
            if equivalent(remote, database.clone()).await? {
                state
                    .official
                    .revision
                    .store(cloud.revision, std::sync::atomic::Ordering::Release);
                return Ok(state.official.set(Status {
                    phase: "ready",
                    cloud_has_data: true,
                    last_synced_at: cloud.updated_at,
                    error: None,
                }));
            }
        }
        let package = envelope::seal(material.clone(), database.clone()).await?;
        let saved = network::put(api, account, cloud.revision, &package).await;
        match saved {
            Ok(saved) => {
                state
                    .official
                    .revision
                    .store(saved.revision, std::sync::atomic::Ordering::Release);
                state
                    .store
                    .save_official_baseline(
                        &api.origin,
                        &account.user.id,
                        saved.revision,
                        &database,
                    )
                    .await
                    .map_err(storage_error)?;
                return Ok(state.official.set(Status {
                    phase: "ready",
                    cloud_has_data: true,
                    last_synced_at: saved.updated_at,
                    error: None,
                }));
            }
            Err(error) if error.code == "ACCOUNT_CONFLICT" => {
                cloud = head(api, account).await?;
            }
            Err(error) => return Err(error),
        }
    }
    Err(failure(
        "SYNC_CONFLICT",
        "其他设备持续修改云端，已保留本地数据，稍后自动重试",
    ))
}

async fn equivalent(left: Zeroizing<Vec<u8>>, right: Zeroizing<Vec<u8>>) -> Result<bool> {
    tokio::task::spawn_blocking(move || {
        terminal_storage::official_sync::equivalent_official_databases(&left, &right)
    })
    .await
    .map_err(|_| failure("SYNC_INTERRUPTED", "同步比较中断"))?
    .map_err(storage_error)
}

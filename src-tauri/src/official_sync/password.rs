//! 官方改密先提交云端CAS，再对齐本地可恢复的加密文件。
use super::{
    envelope, network,
    runtime::Status,
    service::{account, failed, needs_password},
    sync::synchronize,
};
use crate::{
    account::{failure, network::Api},
    state::{AppState, storage_error},
};
use terminal_core::Result;
use zeroize::Zeroizing;
/// 官方改密先以 CAS 更新云端，再切换本地加密文件；两端故障均保留恢复材料。
pub async fn change_password(
    state: &AppState,
    old: Zeroizing<String>,
    new: Zeroizing<String>,
) -> Result<terminal_security::PasswordChangeResult> {
    let _operation = state.official.operation.lock().await;
    let _mode = crate::account::service::begin_mode_change(state).await?;
    let api = Api::new()?;
    let Some(account) = account(state, &api).await? else {
        return state
            .vault
            .change_password(old, new)
            .await
            .map_err(crate::commands::security_error);
    };
    if !state.official.ready() {
        return Err(failure(
            "SYNC_PASSWORD_REQUIRED",
            "请先在主窗口完成官方保险库密码验证",
        ));
    }
    if old == new {
        return Err(failure(
            "VAULT_PASSWORD_UNCHANGED",
            "新密码不能与旧密码相同",
        ));
    }
    state
        .vault
        .verify_password(old)
        .await
        .map_err(crate::commands::security_error)?;
    let material = tokio::task::spawn_blocking(move || {
        use rand_core::{OsRng, RngCore};
        let mut salt = [0; 16];
        OsRng
            .try_fill_bytes(&mut salt)
            .map_err(|_| terminal_security::SecurityError::Vault("无法生成安全密码盐"))?;
        terminal_security::derive_vault_key(&new, salt)
    })
    .await
    .map_err(|_| failure("SYNC_INTERRUPTED", "密码派生中断"))?
    .map_err(crate::commands::security_error)?;
    if let Err(error) = synchronize(state, &api, &account, None).await {
        failed(state, &error, true, state.official.value().cloud_has_data);
        return Err(error);
    }
    let revision = state
        .official
        .revision
        .load(std::sync::atomic::Ordering::Acquire);
    let database = state
        .vault
        .export_backup_database()
        .await
        .map_err(crate::commands::security_error)?;
    let package = envelope::seal(material.clone(), database.clone()).await?;
    let result = network::put(&api, &account, revision, &package).await;
    let saved = match result {
        Ok(saved) => saved,
        Err(error) => {
            // 网络失败不能判断服务器是否已提交；不自动重放或改变本地密码。
            needs_password(state,true,Some("云端改密未能确认，本地仍保留原密码；请在主窗口验证云端密码，网络中断时可能需要新密码".into()));
            return Err(error);
        }
    };
    if let Err(error) = state.vault.adopt_master_key(material).await {
        needs_password(
            state,
            true,
            Some("云端密码已更新，本地切换未完成；请保留本地文件并在主窗口输入新云密码恢复".into()),
        );
        return Err(crate::commands::security_error(error));
    }
    state
        .store
        .save_official_baseline(&api.origin, &account.user.id, saved.revision, &database)
        .await
        .map_err(storage_error)?;
    state
        .official
        .revision
        .store(saved.revision, std::sync::atomic::Ordering::Release);
    state.official.set(Status {
        phase: "ready",
        cloud_has_data: true,
        last_synced_at: saved.updated_at,
        error: None,
    });
    let status = state
        .vault
        .status()
        .await
        .map_err(crate::commands::security_error)?;
    Ok(terminal_security::PasswordChangeResult {
        keychain_warning: status.keychain_warning,
        cleanup_warning: status.cleanup_warning,
    })
}

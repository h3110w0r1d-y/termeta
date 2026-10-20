//! 业务校验、压缩与加密属于阻塞计算，调用方只接收不透明密文。
use crate::{account::failure, commands::security_error};
use terminal_core::Result;
use terminal_security::{
    VaultKey,
    backup::{self, BackupMetadata},
};
use zeroize::Zeroizing;
pub(super) async fn seal(material: VaultKey, database: Zeroizing<Vec<u8>>) -> Result<String> {
    let package = tokio::task::spawn_blocking(move || {
        let metadata = BackupMetadata {
            backup_id: uuid::Uuid::new_v4().to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_millis() as u64)
                .unwrap_or(0),
            device_name: "Termeta desktop".into(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            counts: terminal_storage::inspect_backup_database(database.to_vec())?,
        };
        backup::seal_with_key(&material.key, &material.salt, &metadata, &database)
    })
    .await
    .map_err(|_| failure("SYNC_INTERRUPTED", "同步加密中断"))?
    .map_err(security_error)?;
    if package.len() > 32 * 1024 * 1024 {
        return Err(failure("SYNC_SIZE", "官方云同步数据超过32 MiB限额"));
    }
    Ok(package)
}

//! 固定官方来源上的版本比较协议；写入必须回读确认版本与完整密文。
use crate::account::{failure, network::Api, storage::SavedAccount};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use terminal_core::Result;
#[derive(Deserialize)]
pub(super) struct Head {
    pub(super) revision: i64,
    pub(super) encrypted_content: Option<String>,
    pub(super) updated_at: Option<String>,
}
pub(super) async fn head(api: &Api, account: &SavedAccount) -> Result<Head> {
    let head: Head = api
        .request::<_, ()>(
            Method::GET,
            "api/sync/vault",
            Some(&account.access_token),
            None,
        )
        .await?;
    if head.revision < 0
        || ((head.revision == 0) != head.encrypted_content.is_none())
        || head
            .encrypted_content
            .as_ref()
            .is_some_and(|v| v.len() > 32 * 1024 * 1024)
    {
        return Err(failure("SYNC_RESPONSE", "官方同步版本无效"));
    }
    Ok(head)
}

pub(super) async fn put(
    api: &Api,
    account: &SavedAccount,
    revision: i64,
    package: &str,
) -> Result<Head> {
    #[derive(Serialize)]
    struct Write<'a> {
        base_revision: i64,
        encrypted_content: &'a str,
    }
    let saved: Head = api
        .request(
            Method::PUT,
            "api/sync/vault",
            Some(&account.access_token),
            Some(&Write {
                base_revision: revision,
                encrypted_content: package,
            }),
        )
        .await?;
    validate_commit(&saved, revision, package)?;
    Ok(saved)
}
fn validate_commit(saved: &Head, base_revision: i64, package: &str) -> Result<()> {
    if saved.revision
        != base_revision
            .checked_add(1)
            .ok_or_else(|| failure("SYNC_RESPONSE", "同步版本超出范围"))?
        || saved.encrypted_content.as_deref() != Some(package)
    {
        return Err(failure(
            "SYNC_RESPONSE",
            "官方服务未确认上传内容，已保留本地数据，请刷新同步状态",
        ));
    }
    Ok(())
}

//! 云备份仅传输不透明加密包；格式、密码与业务恢复由上层负责。

mod gist;
mod webdav;

use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

pub const MAX_PACKAGE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_GIST_PACKAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_LIST_BYTES: usize = 4 * 1024 * 1024;
const MAX_BACKUPS: usize = 1000;
const BACKUP_FILENAME: &str = "termeta-backup.txt";

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackupProviderKind {
    Gist,
    Webdav,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupTarget {
    pub provider: BackupProviderKind,
    pub endpoint: Option<String>,
    pub username: Option<String>,
    pub secret: String,
    pub device_name: String,
}

impl Drop for BackupTarget {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBackup {
    pub id: String,
    pub size: Option<u64>,
}

/// 新增 OSS 等存储时实现此边界，不向提供方传递解密密码或数据库信息。
#[async_trait]
pub trait BackupProvider: Send + Sync {
    async fn test(&self) -> Result<(), String>;
    async fn list(&self) -> Result<Vec<RemoteBackup>, String>;
    async fn upload(&self, payload: &[u8]) -> Result<RemoteBackup, String>;
    async fn download(&self, id: &str) -> Result<Vec<u8>, String>;
}

pub fn validate_target(target: &BackupTarget) -> Result<(), String> {
    if target.secret.is_empty() || target.secret.len() > 16 * 1024 {
        return Err("请输入有效的云端认证凭据".into());
    }
    if target.device_name.trim().is_empty()
        || target.device_name.len() > 256
        || target.device_name.chars().any(char::is_control)
    {
        return Err("设备名称须为 1–256 字节的可见文本".into());
    }
    match target.provider {
        BackupProviderKind::Gist => {
            if target.secret.chars().any(char::is_control) {
                return Err("GitHub Token 包含无效字符".into());
            }
        }
        BackupProviderKind::Webdav => {
            webdav::root_url(target)?;
            if target
                .username
                .as_ref()
                .is_none_or(|name| name.is_empty() || name.len() > 1024 || name.contains(':'))
            {
                return Err("请输入有效的 WebDAV 用户名（不可包含冒号）".into());
            }
        }
    }
    Ok(())
}

pub fn provider(target: &BackupTarget) -> Result<Box<dyn BackupProvider>, String> {
    validate_target(target)?;
    let client = Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        // 创建请求不得自动重试，避免网络结果不明时生成重复快照。
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(120))
        .user_agent("Termeta-Backup/1")
        .build()
        .map_err(|_| "无法初始化安全 HTTP 客户端".to_string())?;
    match target.provider {
        BackupProviderKind::Gist => Ok(Box::new(gist::GistProvider::new(client, target.clone()))),
        BackupProviderKind::Webdav => Ok(Box::new(webdav::WebdavProvider::new(
            client,
            target.clone(),
        )?)),
    }
}

fn network_error(error: reqwest::Error) -> String {
    // reqwest 原始错误可能含 URL，禁止把认证地址或远端响应正文传给前端。
    if error.is_timeout() {
        "云备份请求超时；上传可能已成功，请刷新列表核验后再重试".into()
    } else if error.is_connect() {
        "无法建立云备份连接，请检查网络、地址和证书信任".into()
    } else {
        "云备份网络请求失败；上传结果可能需要刷新列表核验".into()
    }
}

fn check_status(status: StatusCode) -> Result<(), String> {
    if status.is_success() {
        return Ok(());
    }
    Err(match status.as_u16() {
        401 => "云端认证失败，请检查认证凭据".into(),
        403 => "云端拒绝访问，请检查权限或 API 额度".into(),
        404 => "云端目录或备份不存在；目录需要预先创建".into(),
        405 => "服务端不支持所需的备份协议方法".into(),
        409 => "云端目录不存在或资源冲突".into(),
        412 => "远端同名对象已存在，已拒绝覆盖".into(),
        413 => "备份包超过服务端大小限制".into(),
        429 => "云端请求过于频繁，请稍后再试".into(),
        507 => "云端存储空间不足".into(),
        300..=399 => "云端返回重定向；请配置最终 HTTPS 地址".into(),
        code => format!("云端请求失败（HTTP {code}）"),
    })
}

async fn read_limited(mut response: Response, limit: usize) -> Result<Vec<u8>, String> {
    check_status(response.status())?;
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err("云端响应超过备份大小限制".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Err("云端响应超过备份大小限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn validate_payload(payload: &[u8], limit: usize) -> Result<&str, String> {
    if payload.is_empty() || payload.len() > limit {
        return Err("加密备份包为空或超过当前提供方的大小限制".into());
    }
    std::str::from_utf8(payload).map_err(|_| "加密备份包必须是 UTF-8 文本".into())
}

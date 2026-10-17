use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::{Client, Method, RequestBuilder};
use serde::Deserialize;
use url::Url;

use super::{
    BACKUP_FILENAME, BackupProvider, BackupTarget, MAX_BACKUPS, MAX_GIST_PACKAGE_BYTES,
    MAX_LIST_BYTES, RemoteBackup, network_error, read_limited, validate_payload,
};

const API_ROOT: &str = "https://api.github.com";
const DESCRIPTION: &str = "Termeta encrypted backup";

pub(super) struct GistProvider {
    client: Client,
    target: BackupTarget,
}

#[derive(Deserialize)]
struct Gist {
    id: String,
    #[serde(default)]
    public: bool,
    #[serde(default)]
    truncated: bool,
    files: HashMap<String, GistFile>,
}

#[derive(Deserialize)]
struct GistFile {
    size: Option<u64>,
    content: Option<String>,
    raw_url: Option<String>,
    #[serde(default)]
    truncated: bool,
}

impl GistProvider {
    pub(super) fn new(client: Client, target: BackupTarget) -> Self {
        Self { client, target }
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client
            .request(method, format!("{API_ROOT}{path}"))
            .bearer_auth(&self.target.secret)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    async fn get_gist(&self, id: &str) -> Result<Gist, String> {
        validate_id(id)?;
        let response = self
            .request(Method::GET, &format!("/gists/{id}"))
            .send()
            .await
            .map_err(network_error)?;
        // JSON 转义可能增加正文尺寸；仍独立限制解码后的文件正文。
        let bytes = read_limited(response, MAX_GIST_PACKAGE_BYTES * 2).await?;
        let gist: Gist = serde_json::from_slice(&bytes)
            .map_err(|_| "GitHub 返回了无效的备份描述".to_string())?;
        if gist.id != id || gist.public || gist.truncated || gist.files.len() != 1 {
            return Err("该 Gist 不是有效的独立 Secret 备份".into());
        }
        Ok(gist)
    }
}

#[async_trait]
impl BackupProvider for GistProvider {
    async fn test(&self) -> Result<(), String> {
        // 只读验证账户认证；不能通过写入临时对象测试权限。
        let response = self
            .request(Method::GET, "/user")
            .send()
            .await
            .map_err(network_error)?;
        read_limited(response, MAX_LIST_BYTES).await?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<RemoteBackup>, String> {
        let mut backups = Vec::new();
        // 不执行服务端 Link 中的任意 URL；只在固定 API 递增页码。
        for page in 1..=100 {
            let response = self
                .request(Method::GET, &format!("/gists?per_page=100&page={page}"))
                .send()
                .await
                .map_err(network_error)?;
            let bytes = read_limited(response, MAX_LIST_BYTES).await?;
            let gists: Vec<Gist> = serde_json::from_slice(&bytes)
                .map_err(|_| "GitHub 返回了无效的备份列表".to_string())?;
            let page_len = gists.len();
            if page_len > 100 {
                return Err("GitHub 返回了超限的分页列表".into());
            }
            for gist in gists {
                if gist.public || gist.truncated || gist.files.len() != 1 {
                    continue;
                }
                if let Some(file) = gist.files.get(BACKUP_FILENAME) {
                    validate_id(&gist.id)?;
                    backups.push(RemoteBackup {
                        id: gist.id,
                        size: file.size,
                    });
                    if backups.len() > MAX_BACKUPS {
                        return Err("云端备份超过 1000 份，请先在提供方管理旧备份".into());
                    }
                }
            }
            if page_len < 100 {
                return Ok(backups);
            }
        }
        Err("GitHub 账户 Gist 数量超出本次扫描上限，无法提供完整备份列表".into())
    }

    async fn upload(&self, payload: &[u8]) -> Result<RemoteBackup, String> {
        let content = validate_payload(payload, MAX_GIST_PACKAGE_BYTES)?;
        let response = self
            .request(Method::POST, "/gists")
            .json(&serde_json::json!({
                "description": DESCRIPTION,
                "public": false,
                "files": { BACKUP_FILENAME: { "content": content } }
            }))
            .send()
            .await
            .map_err(network_error)?;
        let bytes = read_limited(response, MAX_GIST_PACKAGE_BYTES * 2).await?;
        let gist: Gist = serde_json::from_slice(&bytes)
            .map_err(|_| "Gist 创建结果无法解析；远端可能已创建，请刷新列表核验".to_string())?;
        validate_id(&gist.id)?;
        let verified = self
            .download(&gist.id)
            .await
            .map_err(|_| "Gist 已创建，但上传后核验失败；请刷新列表核验后再重试".to_string())?;
        if verified != payload {
            return Err("Gist 已创建，但上传后正文不一致；该备份不能使用".into());
        }
        Ok(RemoteBackup {
            id: gist.id,
            size: Some(payload.len() as u64),
        })
    }

    async fn download(&self, id: &str) -> Result<Vec<u8>, String> {
        let gist = self.get_gist(id).await?;
        let file = gist
            .files
            .get(BACKUP_FILENAME)
            .ok_or_else(|| "该 Gist 不包含 Termeta 备份文件".to_string())?;
        if file
            .size
            .is_some_and(|size| size > MAX_GIST_PACKAGE_BYTES as u64)
        {
            return Err("Gist 备份包超过 8 MiB 限制".into());
        }
        if !file.truncated {
            if let Some(content) = &file.content {
                validate_payload(content.as_bytes(), MAX_GIST_PACKAGE_BYTES)?;
                return Ok(content.as_bytes().to_vec());
            }
        }
        let raw_url = file
            .raw_url
            .as_deref()
            .ok_or_else(|| "GitHub 未提供完整备份文件的读取地址".to_string())?;
        let url = validate_raw_url(raw_url, id)?;
        // raw 请求绝不携带 GitHub Token，也不跟随任何重定向。
        let response = self.client.get(url).send().await.map_err(network_error)?;
        let bytes = read_limited(response, MAX_GIST_PACKAGE_BYTES).await?;
        validate_payload(&bytes, MAX_GIST_PACKAGE_BYTES)?;
        Ok(bytes)
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Gist 备份 ID 无效".into());
    }
    Ok(())
}

fn validate_raw_url(value: &str, id: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "Gist 原始文件地址无效".to_string())?;
    if url.scheme() != "https"
        || url.host_str() != Some("gist.githubusercontent.com")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !url
            .path_segments()
            .is_some_and(|mut parts| parts.any(|part| part == id))
    {
        return Err("拒绝 GitHub 返回的非官方或越界原始文件地址".into());
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_raw_auth_and_cross_origin() {
        assert!(validate_raw_url("https://gist.githubusercontent.com/u/abc/raw/x", "abc").is_ok());
        for value in [
            "https://evil.example/u/abc/raw/x",
            "https://token@gist.githubusercontent.com/u/abc/raw/x",
            "http://gist.githubusercontent.com/u/abc/raw/x",
            "https://gist.githubusercontent.com/u/def/raw/x",
        ] {
            assert!(validate_raw_url(value, "abc").is_err());
        }
        assert!(validate_id("../user").is_err());
        assert!(validate_id("abc?token=x").is_err());
    }
}

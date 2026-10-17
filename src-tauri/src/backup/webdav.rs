use std::collections::HashSet;

use async_trait::async_trait;
use quick_xml::{events::Event, name::ResolveResult, reader::NsReader};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use url::Url;
use uuid::Uuid;

use super::{
    BackupProvider, BackupTarget, MAX_BACKUPS, MAX_LIST_BYTES, MAX_PACKAGE_BYTES, RemoteBackup,
    check_status, network_error, read_limited, validate_payload,
};

const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="UTF-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/><d:getcontentlength/></d:prop></d:propfind>"#;

pub(super) struct WebdavProvider {
    client: Client,
    target: BackupTarget,
    root: Url,
}

impl WebdavProvider {
    pub(super) fn new(client: Client, target: BackupTarget) -> Result<Self, String> {
        let root = root_url(&target)?;
        Ok(Self {
            client,
            target,
            root,
        })
    }

    fn request(&self, method: Method, url: Url) -> RequestBuilder {
        self.client.request(method, url).basic_auth(
            self.target.username.as_deref().unwrap_or_default(),
            Some(&self.target.secret),
        )
    }

    fn object_url(&self, id: &str) -> Result<Url, String> {
        validate_id(id)?;
        self.root.join(id).map_err(|_| "WebDAV 备份路径无效".into())
    }

    async fn propfind(&self, depth: &'static str) -> Result<Vec<u8>, String> {
        let method =
            Method::from_bytes(b"PROPFIND").map_err(|_| "无法创建 WebDAV 请求".to_string())?;
        let response = self
            .request(method, self.root.clone())
            .header("Depth", depth)
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(PROPFIND_BODY)
            .send()
            .await
            .map_err(network_error)?;
        check_status(response.status())?;
        if response.status() != StatusCode::MULTI_STATUS {
            return Err("WebDAV 服务未返回有效的 207 目录响应".into());
        }
        read_limited(response, MAX_LIST_BYTES).await
    }
}

#[async_trait]
impl BackupProvider for WebdavProvider {
    async fn test(&self) -> Result<(), String> {
        let xml = self.propfind("0").await?;
        parse_list(&self.root, &xml)?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<RemoteBackup>, String> {
        let xml = self.propfind("1").await?;
        parse_list(&self.root, &xml)
    }

    async fn upload(&self, payload: &[u8]) -> Result<RemoteBackup, String> {
        validate_payload(payload, MAX_PACKAGE_BYTES)?;
        // 对象名只包含产品标识和随机 UUID，时间及设备名保留在密文中。
        let id = format!("termeta-{}.backup", Uuid::new_v4());
        let response = self
            .request(Method::PUT, self.object_url(&id)?)
            .header("If-None-Match", "*")
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(payload.to_vec())
            .send()
            .await
            .map_err(network_error)?;
        check_status(response.status())?;
        // 只接受新建结果，服务端报告覆盖时明确失败，不能静默降级。
        if response.status() != StatusCode::CREATED {
            return Err("WebDAV 未确认新建对象；请确认服务支持条件 PUT，并刷新列表核验".into());
        }
        let verified = self.download(&id).await.map_err(|_| {
            "WebDAV 对象已创建，但上传后核验失败；请刷新列表核验后再重试".to_string()
        })?;
        if verified != payload {
            return Err("WebDAV 对象已创建，但上传后正文不一致；该备份不能使用".into());
        }
        Ok(RemoteBackup {
            id,
            size: Some(payload.len() as u64),
        })
    }

    async fn download(&self, id: &str) -> Result<Vec<u8>, String> {
        let response = self
            .request(Method::GET, self.object_url(id)?)
            .send()
            .await
            .map_err(network_error)?;
        let bytes = read_limited(response, MAX_PACKAGE_BYTES).await?;
        validate_payload(&bytes, MAX_PACKAGE_BYTES)?;
        Ok(bytes)
    }
}

pub(super) fn root_url(target: &BackupTarget) -> Result<Url, String> {
    let value = target.endpoint.as_deref().unwrap_or_default();
    if value.len() > 4096 || value.chars().any(char::is_control) || value.trim() != value {
        return Err("WebDAV 地址过长或含有无效空白字符".into());
    }
    let mut url =
        Url::parse(value).map_err(|_| "请输入有效的 WebDAV HTTPS 目录地址".to_string())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("WebDAV 须使用 HTTPS 目录，地址不可内嵌凭据、查询或片段".into());
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

fn validate_id(id: &str) -> Result<(), String> {
    let value = id
        .strip_prefix("termeta-")
        .and_then(|value| value.strip_suffix(".backup"))
        .ok_or_else(|| "WebDAV 备份 ID 无效".to_string())?;
    let uuid = Uuid::parse_str(value).map_err(|_| "WebDAV 备份 ID 无效".to_string())?;
    if uuid.to_string() != value || uuid.get_version_num() != 4 {
        return Err("WebDAV 备份 ID 必须使用规范的随机 UUID".into());
    }
    Ok(())
}

fn href_id(root: &Url, href: &str) -> Result<Option<String>, String> {
    if href.len() > 8192 || href.chars().any(char::is_control) {
        return Err("WebDAV 返回了无效的目录 href".into());
    }
    let url = root
        .join(href)
        .map_err(|_| "WebDAV 返回了无效的目录 href".to_string())?;
    if url.origin() != root.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("拒绝 WebDAV 返回的跨来源目录项".into());
    }
    // 目录自身不带尾斜线的响应也合法；所有子项都必须留在配置目录内。
    if url.path().trim_end_matches('/') == root.path().trim_end_matches('/') {
        return Ok(None);
    }
    let suffix = url
        .path()
        .strip_prefix(root.path())
        .ok_or_else(|| "拒绝 WebDAV 返回的越界目录项".to_string())?;
    if suffix.contains('/') || validate_id(suffix).is_err() {
        return Ok(None);
    }
    Ok(Some(suffix.to_string()))
}

#[derive(Default)]
struct DavEntry {
    href: String,
    length: String,
    status: String,
    collection: bool,
}

fn parse_list(root: &Url, bytes: &[u8]) -> Result<Vec<RemoteBackup>, String> {
    let xml =
        std::str::from_utf8(bytes).map_err(|_| "WebDAV 目录必须使用 UTF-8 XML".to_string())?;
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    reader.resolver_mut().set_max_namespace_bindings(128);
    let mut stack = Vec::<String>::new();
    let mut entry: Option<DavEntry> = None;
    let mut backups = Vec::new();
    let mut ids = HashSet::new();
    let mut root_seen = false;
    let mut root_closed = false;
    let mut root_entry_seen = false;
    let mut nodes = 0usize;
    let mut responses = 0usize;
    loop {
        let (namespace, event) = reader
            .read_resolved_event()
            .map_err(|_| "WebDAV 返回了无效的 XML 目录".to_string())?;
        nodes += 1;
        if nodes > 100_000 {
            return Err("WebDAV XML 元素数超过限制".into());
        }
        match event {
            Event::DocType(_) => return Err("WebDAV XML 不允许 DTD 或外部实体".into()),
            Event::Start(element) => {
                let dav = matches!(namespace, ResolveResult::Bound(ns) if ns.0 == "DAV:");
                let name = if dav {
                    element.local_name().into_inner().to_string()
                } else {
                    String::new()
                };
                if stack.is_empty() {
                    if root_seen || root_closed || name != "multistatus" {
                        return Err("WebDAV XML 缺少 DAV multistatus 根节点".into());
                    }
                    root_seen = true;
                }
                if name == "response" {
                    if stack.len() != 1 || entry.is_some() {
                        return Err("WebDAV XML response 层级无效".into());
                    }
                    responses += 1;
                    if responses > 10_000 {
                        return Err("WebDAV 目录项超过扫描限制".into());
                    }
                    entry = Some(DavEntry::default());
                }
                if name == "collection" {
                    if let Some(entry) = entry.as_mut() {
                        entry.collection = true;
                    }
                }
                stack.push(name);
                if stack.len() > 32 {
                    return Err("WebDAV XML 嵌套超过限制".into());
                }
            }
            Event::End(_) => {
                let name = stack
                    .pop()
                    .ok_or_else(|| "WebDAV XML 层级无效".to_string())?;
                if name == "response" {
                    let entry = entry
                        .take()
                        .ok_or_else(|| "WebDAV XML 目录项无效".to_string())?;
                    if entry.href.is_empty() {
                        return Err("WebDAV 目录项缺少 href".into());
                    }
                    if !entry.status.is_empty() {
                        let status = entry
                            .status
                            .split_whitespace()
                            .nth(1)
                            .and_then(|code| code.parse::<u16>().ok())
                            .and_then(|code| StatusCode::from_u16(code).ok())
                            .ok_or_else(|| "WebDAV 目录项状态无效".to_string())?;
                        check_status(status)?;
                    }
                    let href = root
                        .join(entry.href.trim())
                        .map_err(|_| "WebDAV 目录 href 无效".to_string())?;
                    if href.origin() == root.origin()
                        && href.path().trim_end_matches('/') == root.path().trim_end_matches('/')
                    {
                        if !entry.collection {
                            return Err("WebDAV 目标不是可读取的目录".into());
                        }
                        root_entry_seen = true;
                    }
                    if let Some(id) = href_id(root, entry.href.trim())? {
                        if !entry.collection && ids.insert(id.clone()) {
                            backups.push(RemoteBackup {
                                id,
                                size: entry.length.trim().parse().ok(),
                            });
                            if backups.len() > MAX_BACKUPS {
                                return Err("云端备份超过 1000 份，请先在提供方管理旧备份".into());
                            }
                        }
                    }
                }
                if stack.is_empty() {
                    root_closed = true;
                }
            }
            Event::Text(text) => {
                if stack.is_empty() && !text.trim().is_empty() {
                    return Err("WebDAV XML 根节点之外包含无效正文".into());
                }
                append_text(&stack, &mut entry, text.as_ref())?;
            }
            Event::CData(text) => append_text(&stack, &mut entry, text.as_ref())?,
            Event::GeneralRef(reference) => {
                // 只解析 XML 内建实体和字符引用，绝不查询外部资源或扩展用户实体。
                let escaped = format!("&{};", reference.as_ref());
                let text = quick_xml::escape::unescape(&escaped)
                    .map_err(|_| "WebDAV XML 包含未知实体".to_string())?;
                append_text(&stack, &mut entry, &text)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !root_seen || !root_closed || !stack.is_empty() || !root_entry_seen {
        return Err("WebDAV XML 目录不完整".into());
    }
    Ok(backups)
}

fn append_text(stack: &[String], entry: &mut Option<DavEntry>, text: &str) -> Result<(), String> {
    if let Some(entry) = entry.as_mut() {
        let target = match stack.last().map(String::as_str) {
            Some("href") if stack.len() == 3 => Some((&mut entry.href, 8192usize)),
            Some("getcontentlength") => Some((&mut entry.length, 32usize)),
            Some("status") if stack.len() == 3 => Some((&mut entry.status, 128usize)),
            _ => None,
        };
        if let Some((target, limit)) = target {
            if text.len() > limit.saturating_sub(target.len()) {
                return Err("WebDAV XML 字段超过限制".into());
            }
            target.push_str(text);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "termeta-123e4567-e89b-42d3-a456-426614174000.backup";

    #[test]
    fn dav_href_stays_within_configured_directory() {
        let root = Url::parse("https://example.test/dav/backups/").unwrap();
        assert_eq!(
            href_id(&root, &format!("/dav/backups/{ID}")).unwrap(),
            Some(ID.into())
        );
        assert!(href_id(&root, &format!("https://other.test/dav/backups/{ID}")).is_err());
        assert!(href_id(&root, &format!("../{ID}")).is_err());
        assert!(href_id(&root, &format!("/dav/backups-evil/{ID}")).is_err());
        assert!(href_id(&root, &format!("%2e%2e/{ID}")).is_err());
        assert!(validate_id("../file").is_err());
        assert!(validate_id(&format!("{ID}?overwrite=true")).is_err());
    }

    #[test]
    fn parses_bounded_dav_and_rejects_entities() {
        let root = Url::parse("https://example.test/dav/").unwrap();
        let xml = format!(
            r#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>/dav/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response><d:response><d:href>/dav/{ID}</d:href><d:propstat><d:prop><d:getcontentlength>42</d:getcontentlength><d:resourcetype/></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#
        );
        let entries = parse_list(&root, xml.as_bytes()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].size, Some(42));
        assert!(
            parse_list(
                &root,
                b"<!DOCTYPE x [<!ENTITY e SYSTEM 'file:///etc/passwd'>]><x/>"
            )
            .is_err()
        );
        assert!(parse_list(&root, b"<multistatus/>").is_err());
        assert!(parse_list(&root, b"<d:multistatus xmlns:d='DAV:'>").is_err());
        assert!(
            parse_list(
                &root,
                b"<d:multistatus xmlns:d='DAV:'><d:response><d:href>/dav/</d:href><d:status>HTTP/1.1 403 Forbidden</d:status></d:response></d:multistatus>"
            )
            .is_err()
        );
        let deep = format!(
            "<d:multistatus xmlns:d='DAV:'>{}{}</d:multistatus>",
            "<d:x>".repeat(33),
            "</d:x>".repeat(33)
        );
        assert!(parse_list(&root, deep.as_bytes()).is_err());
    }
}

use crate::SecurityError;
/// 保存时限制文本封装与大小；密钥解析和口令解密仍由 SSH 认证流程负责。
pub(crate) fn validate_key_text(text: &str) -> Result<(), SecurityError> {
    if text.len() > 1024 * 1024 || text.contains('\0') {
        return Err(SecurityError::PrivateKey);
    }
    let mut lines = text.trim().lines();
    if lines.next() != Some("-----BEGIN OPENSSH PRIVATE KEY-----")
        || lines.next_back() != Some("-----END OPENSSH PRIVATE KEY-----")
        || !lines.any(|line| !line.trim().is_empty())
    {
        return Err(SecurityError::PrivateKey);
    }
    Ok(())
}

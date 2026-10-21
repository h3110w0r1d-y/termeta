use russh::keys::PrivateKey;
use terminal_core::{Result, SessionError};

/// OpenSSH 直接读取公开部分，其余格式复用认证解析器；只返回公钥，不保留临时口令。
pub fn public_key_from_private_key(text: &str, passphrase: Option<&str>) -> Result<String> {
    let invalid_key = || {
        SessionError::new(
            "INVALID_PRIVATE_KEY",
            "validation",
            "无法读取公钥，请确认私钥完整且格式受支持；若私钥已加密，请核对解密密码后重试",
        )
    };
    if text.len() > 1024 * 1024 {
        return Err(invalid_key());
    }
    let text = text.trim();
    let key = if text.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----") {
        PrivateKey::from_openssh(text).map_err(|_| invalid_key())?
    } else {
        russh::keys::decode_secret_key(text, passphrase).map_err(|_| invalid_key())?
    };
    let mut public = key.public_key().clone();
    // 不将私钥中可能含有用户信息的注释带入可复制的公钥。
    public.set_comment("");
    public.to_openssh().map_err(|_| invalid_key())
}

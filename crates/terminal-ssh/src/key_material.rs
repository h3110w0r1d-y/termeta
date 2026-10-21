use russh::keys::PrivateKey;
use terminal_core::{Result, SessionError};

/// 只导出 OpenSSH 封装中的公钥，不解密私钥或使用口令，也不证明认证可用。
pub fn public_key_from_private_key(text: &str) -> Result<String> {
    let invalid_key = || {
        SessionError::new(
            "INVALID_PRIVATE_KEY",
            "validation",
            "无法读取公钥，请确认保存的是完整且受支持的 OpenSSH 私钥",
        )
    };
    if text.len() > 1024 * 1024 {
        return Err(invalid_key());
    }
    let key = PrivateKey::from_openssh(text.trim()).map_err(|_| invalid_key())?;
    let mut public = key.public_key().clone();
    // 不将私钥中可能含有用户信息的注释带入可复制的公钥。
    public.set_comment("");
    public.to_openssh().map_err(|_| invalid_key())
}

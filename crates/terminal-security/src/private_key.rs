use crate::SecurityError;
/// 保存时限制文本封装与大小；完整解析和口令解密由认证或公钥查看中的 SSH 库负责。
pub(crate) fn validate_key_text(text: &str) -> Result<(), SecurityError> {
    if text.len() > 1024 * 1024 {
        return Err(SecurityError::Vault("私钥正文不能超过 1 MiB"));
    }
    if text.contains('\0') {
        return Err(SecurityError::Vault(
            "私钥正文包含无效字符，请重新粘贴完整文本",
        ));
    }
    let mut lines = text.trim().lines();
    let Some(first) = lines.next() else {
        return Err(SecurityError::PrivateKey);
    };
    // PPK 使用字段和行数封装，没有 PEM 首尾标记；完整解析交给 SSH 库。
    if first.starts_with("PuTTY-User-Key-File-") {
        return if lines.any(|line| !line.trim().is_empty()) {
            Ok(())
        } else {
            Err(SecurityError::PrivateKey)
        };
    }
    let end = match first {
        "-----BEGIN OPENSSH PRIVATE KEY-----" => "-----END OPENSSH PRIVATE KEY-----",
        "-----BEGIN RSA PRIVATE KEY-----" => "-----END RSA PRIVATE KEY-----",
        "-----BEGIN PRIVATE KEY-----" => "-----END PRIVATE KEY-----",
        "-----BEGIN ENCRYPTED PRIVATE KEY-----" => "-----END ENCRYPTED PRIVATE KEY-----",
        "-----BEGIN EC PRIVATE KEY-----" => "-----END EC PRIVATE KEY-----",
        _ => return Err(SecurityError::PrivateKey),
    };
    if lines.next_back() != Some(end) || !lines.any(|line| !line.trim().is_empty()) {
        return Err(SecurityError::PrivateKey);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_all_pem_envelopes_recognized_by_the_ssh_library() {
        for kind in [
            "OPENSSH PRIVATE KEY",
            "RSA PRIVATE KEY",
            "PRIVATE KEY",
            "ENCRYPTED PRIVATE KEY",
            "EC PRIVATE KEY",
        ] {
            for newline in ["\n", "\r\n"] {
                // 仅测试封装，不把占位正文作为真实密钥，也不执行密码学解析。
                let text = format!(
                    "-----BEGIN {kind}-----{newline}AA=={newline}-----END {kind}-----{newline}"
                );
                assert!(validate_key_text(&text).is_ok(), "{kind}");
            }
        }
        for version in [2, 3] {
            assert!(validate_key_text(&format!("PuTTY-User-Key-File-{version}: ssh-rsa\nEncryption: none\nPrivate-Lines: 1\nAA==\nPrivate-MAC: 00")).is_ok());
        }
    }

    #[test]
    fn rejects_incomplete_mismatched_and_oversized_input() {
        for text in [
            "",
            " \n",
            "-----BEGIN RSA PRIVATE KEY-----",
            "-----BEGIN RSA PRIVATE KEY-----\nAA==\n-----END PRIVATE KEY-----",
            "-----BEGIN PRIVATE KEY-----\n-----END PRIVATE KEY-----",
            "-----BEGIN PUBLIC KEY-----\nAA==\n-----END PUBLIC KEY-----",
            "PuTTY-User-Key-File-3: ssh-rsa",
            "\0",
        ] {
            assert!(validate_key_text(text).is_err());
        }
        assert!(matches!(
            validate_key_text(&"A".repeat(1024 * 1024 + 1)),
            Err(SecurityError::Vault("私钥正文不能超过 1 MiB"))
        ));
    }
}

use crate::{Result, StorageError};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::Path,
};
use zeroize::Zeroizing;

const MAX_KEY_BYTES: u64 = 1024 * 1024;

fn open_key(path: &Path) -> Result<File> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| {
        StorageError::Validation("私钥文件不可读、格式不支持或权限不安全，请重新选择")
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_KEY_BYTES {
        return Err(StorageError::Validation(
            "私钥文件不可读、格式不支持或权限不安全，请重新选择",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path).map_err(|_| {
        StorageError::Validation("私钥文件不可读、格式不支持或权限不安全，请重新选择")
    })?;
    let opened = file.metadata().map_err(|_| {
        StorageError::Validation("私钥文件不可读、格式不支持或权限不安全，请重新选择")
    })?;
    if !opened.is_file() || opened.len() > MAX_KEY_BYTES {
        return Err(StorageError::Validation(
            "私钥文件不可读、格式不支持或权限不安全，请重新选择",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // 拒绝其他用户可读的私钥，并确认检查与打开指向同一对象。
        if opened.mode() & 0o077 != 0
            || opened.uid() != unsafe { libc::geteuid() }
            || opened.dev() != metadata.dev()
            || opened.ino() != metadata.ino()
        {
            return Err(StorageError::Validation(
                "私钥文件不可读、格式不支持或权限不安全，请重新选择",
            ));
        }
    }
    Ok(file)
}
/// 仅供原生注册与凭据适配调用，不向 IPC 暴露任意路径读取。
/// 摘要绑定实际读入的有界字节；与 SSH 服务端公钥指纹没有关联。
pub fn read_key_file(
    path: &Path,
    expected_fingerprint: Option<&str>,
) -> Result<(Zeroizing<String>, String)> {
    if expected_fingerprint == Some("") {
        return Err(StorageError::Validation(
            "私钥引用尚未授权内容摘要，请通过原生选择器重新选择并授权",
        ));
    }
    let file = open_key(path)?;
    // 预留完整上限，避免增长重分配留下未受 Zeroizing 管理的旧私钥缓冲。
    let mut bytes = Zeroizing::new(Vec::with_capacity(MAX_KEY_BYTES as usize + 1));
    file.take(MAX_KEY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| {
            StorageError::Validation("私钥文件不可读、格式不支持或权限不安全，请重新选择")
        })?;
    if bytes.len() > MAX_KEY_BYTES as usize {
        return Err(StorageError::Validation(
            "私钥文件不可读、格式不支持或权限不安全，请重新选择",
        ));
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        StorageError::Validation("私钥文件不可读、格式不支持或权限不安全，请重新选择")
    })?;
    if !text.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----") {
        return Err(StorageError::Validation(
            "私钥文件不可读、格式不支持或权限不安全，请重新选择",
        ));
    }
    // 显式编码摘要，保持历史登记的小写十六进制表示，不依赖摘要容器的格式化实现。
    let fingerprint = format!("sha256:{}", hex::encode(Sha256::digest(bytes.as_slice())));
    if expected_fingerprint.is_some_and(|expected| expected != fingerprint) {
        return Err(StorageError::Validation(
            "私钥内容已变更，请通过原生选择器重新选择并授权",
        ));
    }
    Ok((Zeroizing::new(text.to_owned()), fingerprint))
}

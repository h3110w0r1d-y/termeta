use crate::SecurityError;
use std::path::Path;
use zeroize::Zeroizing;

/// 注册与使用共享同一套限大小、no-follow 和权限核验，避免边界漂移。
pub fn validate_key_file(path: &Path) -> Result<(), SecurityError> {
    terminal_storage::read_key_file(path, None)
        .map(|_| ())
        .map_err(|_| SecurityError::KeyFile)
}
pub(crate) fn read_key(path: &Path, fingerprint: &str) -> Result<Zeroizing<String>, SecurityError> {
    terminal_storage::read_key_file(path, Some(fingerprint))
        .map(|(secret, _)| secret)
        .map_err(|_| SecurityError::KeyFile)
}

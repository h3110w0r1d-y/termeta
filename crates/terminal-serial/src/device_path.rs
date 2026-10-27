use terminal_core::{Result, SessionError};

fn invalid_path() -> SessionError {
    SessionError::new(
        "INVALID_CONFIG",
        "opening",
        "请输入有效的串口设备：Unix使用/dev下的字符设备路径，Windows使用COM端口名",
    )
}

#[cfg(unix)]
pub fn resolve(path: &str) -> Result<String> {
    use std::{os::unix::fs::FileTypeExt, path::Path};

    // 允许/dev/serial/by-id等符号链接，但最终目标必须仍是/dev下的字符设备。
    let requested = Path::new(path);
    if !requested.is_absolute() || !requested.starts_with("/dev") {
        return Err(invalid_path());
    }
    let resolved =
        std::fs::canonicalize(requested).map_err(|error| super::open_error(error.into()))?;
    if !resolved.starts_with("/dev")
        || !std::fs::metadata(&resolved)
            .map_err(|error| super::open_error(error.into()))?
            .file_type()
            .is_char_device()
    {
        return Err(invalid_path());
    }
    let resolved = resolved.to_str().ok_or_else(invalid_path)?;
    // serialport随后验证终端ioctl与参数；校验通过前不传输用户输入。
    Ok(resolved.to_owned())
}

#[cfg(windows)]
pub fn resolve(path: &str) -> Result<String> {
    windows_port(path).ok_or_else(invalid_path)
}

#[cfg(windows)]
fn windows_port(path: &str) -> Option<String> {
    let upper = path.to_ascii_uppercase();
    let name = upper.strip_prefix(r"\\.\").unwrap_or(&upper);
    let number = name.strip_prefix("COM")?;
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let number: u32 = number.parse().ok()?;
    (number > 0).then(|| format!("COM{number}"))
}

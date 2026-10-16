use crate::host_key::HostKeyHandler;
use russh::{ChannelMsg, client};
use std::time::Duration;
use terminal_core::CancellationToken;
use tokio::time::{Instant, timeout_at};

const SCRIPTS: &str = include_str!("../../../resources/ssh-os-detection.json");

/// 仅使用内置只读命令；远端返回值不会拼入下一条命令。
pub(crate) async fn detect(
    connection: &client::Handle<HostKeyHandler>,
    software: &str,
    cancel: &CancellationToken,
) -> Option<&'static str> {
    if let Some(icon) = from_software(software) {
        return Some(icon);
    }
    let scripts: serde_json::Value = serde_json::from_str(SCRIPTS).ok()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut budget = 32768;
    let (code, shell) = exec(
        connection,
        scripts["shell"].as_str()?,
        deadline,
        cancel,
        &mut budget,
    )
    .await?;
    let script = if code == 127 {
        "routeros"
    } else if code == 0 && shell.trim().to_ascii_lowercase().ends_with("fish") {
        "fish"
    } else if code == 0 && shell.trim().is_empty() {
        "windows"
    } else if code == 0 {
        "posix"
    } else {
        return None;
    };
    let (code, data) = exec(
        connection,
        scripts[script].as_str()?,
        deadline,
        cancel,
        &mut budget,
    )
    .await?;
    if code == 0 {
        if let Some(icon) = parse(&data) {
            return Some(icon);
        }
    }
    // 原生Windows SSH可能没有SHELL；只执行固定的Windows版本查询。
    if script == "posix" {
        let (code, data) = exec(
            connection,
            scripts["windows"].as_str()?,
            deadline,
            cancel,
            &mut budget,
        )
        .await?;
        if code == 0 {
            return parse(&data);
        }
    }
    None
}

async fn exec(
    connection: &client::Handle<HostKeyHandler>,
    command: &str,
    deadline: Instant,
    cancel: &CancellationToken,
    budget: &mut usize,
) -> Option<(u32, String)> {
    let mut channel = tokio::select! {
        _ = cancel.cancelled() => return None,
        result = timeout_at(deadline, connection.channel_open_session()) => result.ok()?.ok()?,
    };
    let operation = async {
        channel.exec(true, command).await.ok()?;
        let mut accepted = false;
        let mut exit_code = None;
        let mut output = Vec::new();
        loop {
            match channel.wait().await {
                Some(ChannelMsg::Success) => accepted = true,
                Some(ChannelMsg::Failure | ChannelMsg::ExitSignal { .. }) => return None,
                Some(ChannelMsg::Data { data }) => {
                    *budget = budget.checked_sub(data.len())?;
                    output.extend_from_slice(&data);
                }
                Some(ChannelMsg::ExtendedData { data, .. }) => {
                    // 标准错误同样计入预算，但不混入识别正文。
                    *budget = budget.checked_sub(data.len())?;
                }
                Some(ChannelMsg::ExitStatus { exit_status }) => exit_code = Some(exit_status),
                Some(ChannelMsg::Close) | None => {
                    return if accepted {
                        Some((exit_code?, String::from_utf8(output).ok()?))
                    } else {
                        None
                    };
                }
                _ => {}
            }
        }
    };
    let result = tokio::select! {
        _ = cancel.cancelled() => None,
        result = timeout_at(deadline, operation) => result.ok().flatten(),
    };
    // 超时、超限及取消仅回收探测channel，不关闭主终端transport。
    let _ = tokio::time::timeout(Duration::from_millis(100), channel.close()).await;
    result
}

fn normalize(value: &str) -> Option<&'static str> {
    match value
        .trim()
        .trim_matches(['"', '\''])
        .to_ascii_lowercase()
        .as_str()
    {
        "darwin" | "macos" | "osx" => Some("osx"),
        "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" | "sles" | "sled" | "suse" => {
            Some("suse")
        }
        "rhel" | "redhat" | "red hat enterprise linux" => Some("redhat"),
        "almalinux" | "alma" => Some("alma"),
        "amzn" | "amazon" | "amazon linux" => Some("amazon"),
        "archlinux" | "arch linux" | "arch" => Some("arch"),
        "mikrotik" | "routeros" => Some("routeros"),
        "raspbian" => Some("raspbian"),
        "centos" => Some("centos"),
        "ubuntu" => Some("ubuntu"),
        "debian" => Some("debian"),
        "openbsd" => Some("openbsd"),
        "netbsd" => Some("netbsd"),
        "freebsd" => Some("freebsd"),
        "fedora" => Some("fedora"),
        "mageia" => Some("mageia"),
        "gentoo" => Some("gentoo"),
        "linux" => Some("linux"),
        "android" => Some("android"),
        "windows" => Some("windows"),
        "cisco" => Some("cisco"),
        "gloria-ops-agent" => Some("gloria_ops_agent"),
        "rocky" | "rocky linux" => Some("rocky"),
        "alpine" => Some("alpine"),
        _ => None,
    }
}

fn from_software(value: &str) -> Option<&'static str> {
    let value = value.to_ascii_lowercase();
    // 明确的平台标识优先于OpenSSH等通用软件名，避免win/red子串误判。
    for (needle, icon) in [
        ("openssh_for_windows", "windows"),
        ("routeros", "routeros"),
        ("mikrotik", "routeros"),
        ("cisco", "cisco"),
        ("gloria-ops-agent", "gloria_ops_agent"),
    ] {
        if value.contains(needle) {
            return Some(icon);
        }
    }
    // 发行版常出现在OpenSSH的版本后缀或注释，按完整词匹配而非任意子串。
    let words: Vec<_> = value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .collect();
    for word in words.iter().filter(|word| **word != "linux") {
        if let Some(icon) = normalize(word) {
            return Some(icon);
        }
    }
    if words.contains(&"linux") {
        return Some("linux");
    }
    None
}

fn parse(data: &str) -> Option<&'static str> {
    // 按os-release字段解析，ID优先，不能用整段子串将Ubuntu误识别为Debian。
    let fields: Vec<_> = data
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    for key in ["ID", "NAME", "DISTRIB_ID"] {
        if let Some((_, value)) = fields.iter().find(|(name, _)| name.trim() == key) {
            if let Some(icon) = normalize(value) {
                return Some(icon);
            }
        }
    }
    if let Some((_, value)) = fields.iter().find(|(name, _)| name.trim() == "ID_LIKE") {
        for id in value.trim_matches(['"', '\'']).split_whitespace() {
            if let Some(icon) = normalize(id) {
                return Some(icon);
            }
        }
    }
    for line in data.lines() {
        if let Some(icon) = normalize(line) {
            return Some(icon);
        }
        let line = line.trim().to_ascii_lowercase();
        if line.starts_with("msys_nt-")
            || line.starts_with("mingw")
            || line.starts_with("cygwin_nt-")
            || line.starts_with("microsoft windows [version")
            || line.starts_with("microsoft windows [版本")
        {
            return Some("windows");
        }
    }
    // 未知发行版仍属于Linux；读取不到发行版文件时uname输出也能降级。
    if fields
        .iter()
        .any(|(key, _)| matches!(key.trim(), "ID" | "PRETTY_NAME"))
    {
        return Some("linux");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_id_precedes_family_and_banner_text() {
        assert_eq!(
            parse("Welcome Debian\nID=ubuntu\nID_LIKE=debian\n"),
            Some("ubuntu")
        );
        assert_eq!(parse("ID=almalinux\nNAME=\"AlmaLinux\""), Some("alma"));
        assert_eq!(parse("ID=unknown\nID_LIKE=\"rhel fedora\""), Some("redhat"));
        assert_eq!(parse("ID=custom"), Some("linux"));
        assert_eq!(parse("Darwin\r\n"), Some("osx"));
        assert_eq!(parse("MikroTik\r\n"), Some("routeros"));
        assert_eq!(parse("Microsoft Windows [Version 10.0.1]"), Some("windows"));
        assert_eq!(parse("permission denied"), None);
    }
    #[test]
    fn generic_ssh_version_is_not_an_os() {
        assert_eq!(from_software("SSH-2.0-OpenSSH_9.9"), None);
        assert_eq!(
            from_software("SSH-2.0-OpenSSH_for_Windows_9.9"),
            Some("windows")
        );
        assert_eq!(from_software("SSH-2.0-Darwinish-red-server"), None);
    }
}

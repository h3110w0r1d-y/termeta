use crate::{Result, SshConfig};
use async_trait::async_trait;

/// 系统图标仅接受内置标识，不接受文件路径、SVG正文或远端返回的任意字符串。
pub const HOST_SYSTEM_ICONS: &[&str] = &[
    "ssh",
    "raspbian",
    "centos",
    "osx",
    "ubuntu",
    "openbsd",
    "netbsd",
    "freebsd",
    "suse",
    "fedora",
    "mageia",
    "arch",
    "gentoo",
    "debian",
    "linux",
    "android",
    "routeros",
    "redhat",
    "windows",
    "cisco",
    "gloria_ops_agent",
    "rocky",
    "alma",
    "alpine",
    "amazon",
    "pi",
    "aws",
    "digitalocean",
];

/// 由壳注入持久化端口；SSH协议层不依赖数据库或Tauri。
#[async_trait]
pub trait RemoteSystemStore: Send + Sync {
    /// 先原子登记尝试，保证并发连接、失败和进程重启均不重复探测。
    async fn claim(&self, profile_id: &str, token: &str, config: &SshConfig) -> Result<bool>;
    /// 只更新仍属于本次目标且未被手动选择覆盖的图标。
    async fn complete(
        &self,
        profile_id: &str,
        token: &str,
        config: &SshConfig,
        icon: Option<&str>,
    ) -> Result<()>;
}

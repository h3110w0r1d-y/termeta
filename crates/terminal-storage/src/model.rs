use serde::{Deserialize, Serialize};
use terminal_core::{AuthStep, SerialConfig, SshStartup};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDraft {
    #[serde(default)]
    pub startup: SshStartup,
    #[serde(default = "default_host_kind")]
    pub kind: String,
    #[serde(default)]
    pub serial_config: Option<SerialConfig>,
    #[serde(default = "default_host_icon")]
    pub icon: String,
    pub id: Option<String>,
    pub revision: Option<i64>,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub username: String,
    pub group_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub auth_plan: Vec<AuthStep>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostProfile {
    pub sort_order: i64,
    #[serde(default)]
    pub startup: SshStartup,
    #[serde(default)]
    pub workspace: Option<SavedWorkspace>,
    #[serde(default = "default_host_kind")]
    pub kind: String,
    #[serde(default)]
    pub serial_config: Option<SerialConfig>,
    pub icon: String,
    pub system_detection_attempted: bool,
    pub id: String,
    pub revision: i64,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub username: String,
    pub group_id: Option<String>,
    pub tags: Vec<String>,
    pub auth_plan: Vec<AuthStep>,
    pub has_password: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostGroup {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub sort_order: i64,
    pub revision: i64,
}

/// 主机与分组必须属于同一读取快照，避免前端组合出已失效的关联。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInventory {
    pub hosts: Vec<HostProfile>,
    pub groups: Vec<HostGroup>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDraft {
    pub id: Option<String>,
    pub revision: Option<i64>,
    pub parent_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedHostKey {
    pub id: String,
    pub canonical_host: String,
    pub port: u16,
    pub key_algorithm: String,
    pub public_key: Vec<u8>,
    pub fingerprint_sha256: String,
    pub trusted_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSetting {
    pub value_version: i64,
    pub value: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyFile {
    pub id: String,
    pub path: String,
    pub display_name: String,
    #[serde(skip)]
    pub(crate) content_fingerprint: String,
}
impl KeyFile {
    pub fn content_fingerprint(&self) -> &str {
        &self.content_fingerprint
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRecord {
    pub id: String,
    #[serde(skip)]
    pub owner_profile_id: Option<String>,
    pub kind: String,
    pub label: String,
    pub status: String,
}

fn default_host_icon() -> String {
    "ssh".into()
}

fn default_host_kind() -> String {
    "ssh".into()
}

/// 保存布局只含主机引用，不保存会话、终端缓冲、认证或广播状态。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum SavedLayout {
    Connection {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    Split {
        axis: String,
        ratio: f64,
        first: Box<SavedLayout>,
        second: Box<SavedLayout>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedWorkspace {
    pub root: SavedLayout,
    pub focused_index: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceDraft {
    pub name: String,
    pub workspace: SavedWorkspace,
    pub expected_revision: Option<i64>,
}

/// 排序携带完整清单版本，迁组与顺序必须在同一事务中校验和保存。
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostArrangement {
    pub ordered_hosts: Vec<HostOrderEntry>,
    pub move_to: Option<HostMove>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostOrderEntry {
    pub id: String,
    pub revision: i64,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostMove {
    pub host_ids: Vec<String>,
    pub group_id: Option<String>,
}

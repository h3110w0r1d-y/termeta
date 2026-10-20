//! 云快照共享的白名单表定义和受控单元格；与活动业务表迁移独立。
use super::invalid;
use crate::Result;
use zeroize::Zeroizing;

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Cell {
    Null,
    Integer(i64),
    Text(Zeroizing<String>),
    Blob(Zeroizing<Vec<u8>>),
}
impl Cell {
    pub(crate) fn text(&self) -> Result<&str> {
        if let Self::Text(value) = self {
            Ok(value)
        } else {
            Err(invalid())
        }
    }
    pub(crate) fn integer(&self) -> Result<i64> {
        if let Self::Integer(value) = self {
            Ok(*value)
        } else {
            Err(invalid())
        }
    }
    pub(crate) fn optional_text(&self) -> Result<Option<&str>> {
        if matches!(self, Self::Null) {
            Ok(None)
        } else {
            self.text().map(Some)
        }
    }
    pub(crate) fn blob(&self) -> Result<&[u8]> {
        if let Self::Blob(value) = self {
            Ok(value)
        } else {
            Err(invalid())
        }
    }
    pub(crate) fn set_text(&mut self, value: String) {
        *self = Self::Text(Zeroizing::new(value));
    }
    pub(crate) fn size(&self) -> usize {
        match self {
            Self::Text(v) => v.len(),
            Self::Blob(v) => v.len(),
            _ => 8,
        }
    }
}
pub(crate) struct Table {
    pub name: &'static str,
    pub(crate) definition: &'static str,
}
impl Table {
    pub(crate) fn columns(&self) -> Vec<&str> {
        self.definition
            .split(',')
            .map(|column| column.split_whitespace().next().unwrap())
            .collect()
    }
    pub(crate) fn schema(&self) -> String {
        format!("CREATE TABLE {} ({})", self.name, self.definition)
    }
    pub(crate) fn select(&self) -> String {
        format!("SELECT {} FROM {}", self.columns().join(","), self.name)
    }
    pub(crate) fn insert(&self) -> String {
        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.name,
            self.columns().join(","),
            vec!["?"; self.columns().len()].join(",")
        )
    }
}
pub(crate) static TABLES: &[Table] = &[
    Table {
        name: "host_groups",
        definition: "id TEXT NOT NULL,parent_id TEXT,name TEXT NOT NULL,sort_order INTEGER NOT NULL,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL",
    },
    Table {
        name: "connection_profiles",
        definition: "id TEXT NOT NULL,kind TEXT NOT NULL,name TEXT NOT NULL,group_id TEXT,favorite INTEGER NOT NULL,config_version INTEGER NOT NULL,config_json TEXT NOT NULL,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL",
    },
    Table {
        name: "credentials",
        definition: "id TEXT NOT NULL,kind TEXT NOT NULL,label TEXT NOT NULL,owner_profile_id TEXT,secret TEXT NOT NULL,status TEXT NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL",
    },
    Table {
        name: "ssh_profiles",
        definition: "profile_id TEXT NOT NULL,address TEXT NOT NULL,canonical_host TEXT NOT NULL,port INTEGER NOT NULL,username TEXT NOT NULL",
    },
    Table {
        name: "profile_auth_steps",
        definition: "id TEXT NOT NULL,profile_id TEXT NOT NULL,sort_order INTEGER NOT NULL,method TEXT NOT NULL,credential_id TEXT",
    },
    Table {
        name: "tags",
        definition: "id TEXT NOT NULL,name TEXT NOT NULL",
    },
    Table {
        name: "profile_tags",
        definition: "profile_id TEXT NOT NULL,tag_id TEXT NOT NULL",
    },
    Table {
        name: "port_forwards",
        definition: "id TEXT NOT NULL,name TEXT NOT NULL,profile_id TEXT,rule_json TEXT NOT NULL,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL",
    },
    Table {
        name: "known_hosts",
        definition: "id TEXT NOT NULL,canonical_host TEXT NOT NULL,port INTEGER NOT NULL,key_algorithm TEXT NOT NULL,public_key BLOB NOT NULL,fingerprint_sha256 TEXT NOT NULL,trusted_at INTEGER NOT NULL,last_seen_at INTEGER",
    },
    Table {
        name: "snippets",
        definition: "id TEXT NOT NULL,name TEXT NOT NULL,content TEXT NOT NULL,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL",
    },
];
pub(crate) type Data = Vec<Vec<Vec<Cell>>>;

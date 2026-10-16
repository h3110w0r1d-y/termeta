-- 独立转发规则引用 SSH 主机；删除主机保留规则供重新选择。
CREATE TABLE port_forwards (
 id TEXT PRIMARY KEY NOT NULL,
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 128),
 profile_id TEXT REFERENCES connection_profiles(id) ON DELETE SET NULL,
 rule_json TEXT NOT NULL,
 revision INTEGER NOT NULL DEFAULT 1,
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
);
CREATE INDEX forwards_profile ON port_forwards(profile_id);
PRAGMA user_version = 4;

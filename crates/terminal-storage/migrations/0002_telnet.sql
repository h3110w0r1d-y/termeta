-- 由 prepare 在独占初始化连接上关闭外键后执行，SQLx 将本脚本及迁移登记包在同一事务。
-- 保留旧迁移校验和；重建父表期间不能触发子表的 ON DELETE CASCADE/SET NULL。
CREATE TABLE connection_profiles_next (
 id TEXT PRIMARY KEY NOT NULL, kind TEXT NOT NULL DEFAULT 'ssh' CHECK(kind IN ('ssh','local','serial','telnet')),
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 128),
 group_id TEXT REFERENCES host_groups(id) ON DELETE SET NULL,
 favorite INTEGER NOT NULL CHECK(favorite IN (0,1)), config_version INTEGER NOT NULL DEFAULT 1,
 config_json TEXT NOT NULL DEFAULT '{}', revision INTEGER NOT NULL DEFAULT 1,
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
INSERT INTO connection_profiles_next SELECT * FROM connection_profiles;
DROP TABLE connection_profiles;
ALTER TABLE connection_profiles_next RENAME TO connection_profiles;
CREATE INDEX profiles_group ON connection_profiles(group_id,name);
PRAGMA user_version = 3;

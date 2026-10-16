-- 仍在专用初始化连接上关闭外键后重建，保留所有子表引用。
CREATE TABLE connection_profiles_next (
 id TEXT PRIMARY KEY NOT NULL, kind TEXT NOT NULL DEFAULT 'ssh' CHECK(kind IN ('ssh','local','serial','telnet','workspace')),
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

-- 历史同名项保留最早一项，其他项加序号；不同候选的序号同余类互斥，避开全部原有名称。
CREATE TEMP TABLE profile_name_changes AS
WITH RECURSIVE
ranked AS (
 SELECT id,trim(name) AS name,
 row_number() OVER (PARTITION BY trim(name) ORDER BY created_at,id) AS duplicate_rank,
 row_number() OVER (ORDER BY created_at,id) AS ordinal,
 count(*) OVER () AS total FROM connection_profiles
 WHERE kind IN ('ssh','telnet','serial','workspace')
),
candidates(id,base,n,total,candidate) AS (
 SELECT id,substr(name,1,96),ordinal,total,substr(name,1,96)||' ('||ordinal||')'
 FROM ranked WHERE duplicate_rank>1
 UNION ALL
 SELECT id,base,n+total,total,base||' ('||(n+total)||')' FROM candidates
 WHERE EXISTS(SELECT 1 FROM ranked WHERE name=candidates.candidate)
)
SELECT id,candidate AS name FROM candidates
WHERE NOT EXISTS(SELECT 1 FROM ranked WHERE name=candidates.candidate);
UPDATE connection_profiles SET name=(SELECT name FROM profile_name_changes WHERE id=connection_profiles.id),revision=revision+1
 WHERE id IN (SELECT id FROM profile_name_changes);
DROP TABLE profile_name_changes;
UPDATE connection_profiles SET name=trim(name) WHERE kind IN ('ssh','telnet','serial','workspace');
CREATE UNIQUE INDEX profiles_unique_name ON connection_profiles(name) WHERE kind IN ('ssh','telnet','serial','workspace');
PRAGMA user_version = 5;

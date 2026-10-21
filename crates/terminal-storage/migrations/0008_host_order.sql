-- 首次升级保留原有收藏/名称顺序，新保存项默认排在手动顺序之后。
ALTER TABLE connection_profiles ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 2147483647;
WITH ordered AS (
 SELECT id, ROW_NUMBER() OVER (ORDER BY favorite DESC, name COLLATE NOCASE, id) - 1 AS position
 FROM connection_profiles WHERE kind IN ('ssh','telnet','serial','workspace')
)
UPDATE connection_profiles SET sort_order = (SELECT position FROM ordered WHERE ordered.id = connection_profiles.id)
WHERE id IN (SELECT id FROM ordered);
PRAGMA user_version = 9;

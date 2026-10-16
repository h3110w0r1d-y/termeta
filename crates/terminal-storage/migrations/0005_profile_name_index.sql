-- 已应用的迁移不得改写；在新事务中升级名称索引，保留全部业务记录与迁移历史。
DROP INDEX profiles_unique_name;
CREATE UNIQUE INDEX profiles_unique_name ON connection_profiles(trim(name)) WHERE kind IN ('ssh','telnet','serial','workspace');
PRAGMA user_version = 6;

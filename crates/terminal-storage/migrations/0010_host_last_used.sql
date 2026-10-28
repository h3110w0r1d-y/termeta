-- 最近使用仅为本机浏览元数据，与手动顺序和配置修改时间相互独立。
ALTER TABLE connection_profiles ADD COLUMN last_used_at INTEGER;
PRAGMA user_version = 11;

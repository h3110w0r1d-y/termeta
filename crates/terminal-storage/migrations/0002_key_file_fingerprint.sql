-- 已有引用不自动获得内容授权，空摘要会使读取失败，用户需重新选择私钥。
ALTER TABLE key_files ADD COLUMN content_fingerprint TEXT NOT NULL DEFAULT '';

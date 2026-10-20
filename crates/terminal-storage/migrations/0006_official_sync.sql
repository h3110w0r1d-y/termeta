-- 官方同步基线仅保存在本机 SQLCipher，不进入业务快照。
CREATE TABLE official_sync_state (
 origin TEXT NOT NULL, account_id TEXT NOT NULL,
 revision INTEGER NOT NULL, baseline BLOB NOT NULL,
 PRIMARY KEY(origin,account_id)
);

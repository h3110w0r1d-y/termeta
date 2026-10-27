-- 云账号只保留绑定身份；密码、访问令牌和云端正文不进入本机存储或备份。
CREATE TABLE cloud_account_bindings (
    id TEXT PRIMARY KEY NOT NULL,
    origin TEXT NOT NULL,
    server_id TEXT NOT NULL,
    client_user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    display_name TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    UNIQUE(origin, client_user_id)
);
PRAGMA user_version = 10;

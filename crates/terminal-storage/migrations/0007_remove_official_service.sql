-- 清理已停用服务的本机认证和同步基线，保留业务数据与第三方备份设置。
DELETE FROM settings WHERE key = 'account';
DROP TABLE official_sync_state;
PRAGMA user_version = 8;

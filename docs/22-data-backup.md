# 数据备份与云端恢复

更新：2026-10-27。当前支持在已解锁后的设置中使用 Gist / WebDAV 手动上传、版本选择与验证后直接新增合并。第三方自动备份、版本清理、双向同步、逐项恢复、本地备份文件入口与主动取消 IPC 尚未实现。

## 1. 范围与操作语义

设置的“备份 / 恢复”页面顶部直接提供带图标的“备份”“恢复”按钮。备份点击后创建独立对象，不修改旧备份；恢复先列出已解密的时间、设备、应用版本和数量，选定版本并点击“恢复”后验证并直接新增合并。两种操作均无二次确认；恢复不按远端时间覆盖本机，也不启动导入的连接或转发。

| 包含 | 排除 |
| --- | --- |
| SSH/Telnet/Serial 主机、分组、标签、保存工作区、转发规则与 Snippets | 应用设置、快捷键、自定义配色及主机配色映射、会话日志与旧连接历史 |
| 公共及主机专属凭据、已导入私钥正文与口令、认证关联 | 私钥来源路径登记、OS 主密钥、vault.json、SQLx 迁移记录 |
| SSH 主机信任 | 云配置/Token/WebDAV 密码/备份密码、OTP、终端输出、运行标签/广播及文件任务/正文 |

快照是白名单业务数据重建的内存 SQLite，不是活动 SQLCipher 文件复制，也不含设置表空页或删除残留。物理备份与元数据配对规则见 [存储](07-storage.md#4-物理备份与恢复边界)。

## 2. 快照与导入事务

`terminal-storage` 在同一读事务读取十表：`host_groups`、`connection_profiles`、`credentials`、`ssh_profiles`、`profile_auth_steps`、`tags`、`profile_tags`、`port_forwards`、`known_hosts`、`snippets`。凭据操作门闩协调多步写入与改密，只导出 ready 凭据并移除进行中的系统识别令牌。事务释放后才压缩、派生密码和联网。

快照在独立内存连接创建固定普通表，经 SQLite serialize 输出真实数据库；独立 `user_version=2`（兼容读取不含 Snippets 的版本 1 九表快照），与业务 schema 9 分开演进，明文不写临时文件。主机手动浏览顺序属于本机元数据，不进入白名单云包，新增合并项排在本机手动顺序之后。已移除的收藏只保留历史 `favorite` 列以兼容旧包，不恢复收藏功能或排序优先级。最大 128 MiB、十表合计 100000 行。全量处理需要内存，不能保证压缩后满足云端限额。

恢复只执行应用内固定 SQL，严格匹配 SQLite 文件头、版本、表结构、字段类型及业务关联，拒绝额外表、视图、触发器与虚拟表。解析采用只读 deserialize、defensive、禁 trusted schema/扩展及只读 authorizer，并限制 SQL 长度、深度、行数和执行预算；认证通过不等于内容可信。

导入在一个目标库事务中完成：

- 主机、分组、凭据、认证步骤、Snippets 及转发生成新 ID，重写关联和工作区叶子；来源失效引用保持失效，不误绑定目标库同 ID 主机。
- 保存项名称冲突追加 ` (同步 n)`，标签按名称复用，既有主机/凭据/转发不更新或删除。
- 信任核验算法、完整公钥及 SHA256 指纹；同端点同公钥跳过，不同公钥保留本机 pin 并警告。
- 恢复不写本机设置或云认证；必须先创建并解锁本机库，再在设置中配置第三方目标。

重复选择同一备份重新导入会产生副本。恢复令牌绑定窗口和配置代次，5 分钟过期且一次性消费；每窗口最多一份明文快照，窗口销毁或配置变更清除待恢复快照和成功摘要。成功摘要同样校验配置代次，只提供进程内短期应答重试，不是跨进程幂等账本。

选定版本后由前端连续请求验证和导入；导入时先取得操作许可并消费令牌，再把许可和明文快照移入独立后端任务；窗口关闭或 IPC 等待者消失不取消已接受的导入。事务成功后登记同代次摘要，失败仍遵守单事务回滚，不把等待者中断解释为可重复提交。

## 3. 包格式与密码

```text
Base64(
  "TMETABK1" [8 bytes] + salt [16 bytes] + nonce [12 bytes]
  + AES-256-GCM(
      metadataLength [u32 big-endian] + metadata JSON
      + databaseLength [u64 big-endian] + Zstd(SQLite snapshot),
      AAD = magic + salt + nonce
    )
)
```

Base64 文本每行最多 64 个字符，每行末尾写入 LF（`\n`），包括最后一行。读取兼容旧的单行包及 LF/CRLF 分行包，仅忽略换行字符；包大小限制包含换行字节，内部二进制包格式与加密方式保持不变。

先以 Zstd level 3 压缩快照，再将其与元数据一起加密。元数据为备份 UUID、毫秒 Unix 时间、设备名称、应用版本及主机/分组/凭据/转发/信任数量，全部位于密文内；云方仍可观察大小、请求和平台创建时间。

独立备份密码使用固定 Argon2id v19（64 MiB、3 轮、1 并行度）派生 32 字节密钥，每包随机盐和 nonce。密码至少 12 个字符、最多 1024 UTF-8 字节，不 trim；本机主密钥和云认证均不代替第三方备份密码。密码保存在本机 SQLCipher 设置，不进入云包，也不传给提供方。

修改备份密码只影响新包；读取旧包需切回旧密码。修改保险库密码或云认证不改变第三方旧包密码。新设备必须输入原备份密码，云包不包含本机密码配置。

解密先认证，再按有界元数据、声明长度和 Zstd 窗口/输出限制解压，核验实际长度、结构及数量。元数据最多 16 KiB，快照/解压输出最多 128 MiB，安全模块包解析最多 192 MiB；云入口另有更小限制。应用控制的短期秘密和明文使用 Zeroizing，不能保证第三方内部副本全部擦除。

列表为显示元数据必须逐包下载解密，每次默认 10 份、最多 20 份；单包失败不阻塞其余条目。

## 4. 提供方与网络边界

`BackupProvider` 仅处理不透明加密包的 `test/list/upload/download`，不接触备份密码或业务导入。新增厂商通过配置校验、适配器与工厂分支扩展，不改变快照或导入事务；当前只实现 Gist/WebDAV。

HTTP 强制 HTTPS、系统证书验证、禁重定向与自动重试，连接 15 秒、单次读取 30 秒、单请求总计 120 秒；分页和逐包列表包含多个请求。读取流按实际字节限额，不能只信 Content-Length。测试只读，上传后 GET 回读比较完整包。创建后核验失败或超时可能已生成备份，应先刷新检查，不能自动重试上传。

| 提供方 | 创建与读取约束 | 限额 |
| --- | --- | --- |
| GitHub Secret Gist | 官方 API + Token，每次创建独立 `public:false` Gist，固定文件 `termeta-backup.txt`。只读取当前账户候选；截断正文仅从受校验的 HTTPS `gist.githubusercontent.com` 获取，不携带 Token | 包 8 MiB，最多扫描 10000 个 Gist、保留 1000 份候选 |
| WebDAV | 用户预建 HTTPS 目录 + Basic 认证。Depth:0 测试、Depth:1 列表；XML 校验 DAV 命名空间/集合/层级，拒绝 DTD、未知实体、跨来源或越界 href。随机 UUID 文件名，`PUT If-None-Match:*` 且要求 201，不降级覆盖，下载 ID 不能指定其他路径 | 包 32 MiB，列表 4 MiB，最多扫描 10000 项、保留 1000 份候选 |

Gist 文件名/描述不写设备和备份时间；Secret Gist 的保密依赖整包加密。WebDAV 服务须支持条件 PUT。

## 5. 配置与权限

本机 `settings.key=backup`、`value_version=2` 保存 `{target:{provider,endpoint,username,secret,deviceName},backupPassword}`，与普通 app 设置分开。专用读取只返回 `hasSecret/hasBackupPassword`，不返回已存秘密。省略秘密仅在提供方、地址和用户名相同时复用，目标变化必须重新输入；上传/列表/下载使用保存的密码。

版本 1 配置可保留认证并补填备份密码后写版本 2，不新增数据库迁移；缺密码时可读配置和测试连接，不能备份或解密。测试可使用配置草稿，实际上传、列表与恢复只使用已保存的配置和密码。

第三方备份要求已解锁，仅在设置内使用。不能用第三方包初始化或绕过既有库解锁。

七个命令均经 AppManifest、ACL 与已解锁的本地 main/settings 调用窗口校验，不扩大通用会话 owner。数据库、已存凭据和包正文不经过前端。单操作信号量在忙碌时直接拒绝，不排队积累大数据；短期表锁不跨网络或数据库等待。当前没有逐字节进度或主动取消命令。

## 6. 源码与参考资料

| 入口 | 职责 |
| --- | --- |
| [storage backup.rs](../crates/terminal-storage/src/backup.rs) / [backup_sqlite.rs](../crates/terminal-storage/src/backup_sqlite.rs) | 白名单快照、结构/业务校验、ID 重写与导入事务 |
| [security backup.rs](../crates/terminal-security/src/backup.rs) | 压缩、密码派生、加密包及凭据门闩 |
| [壳 backup](../src-tauri/src/backup/) / [backup_runtime.rs](../src-tauri/src/backup_runtime.rs) | Gist/WebDAV、网络限额、配置及恢复令牌 |
| [commands/backup.rs](../src-tauri/src/commands/backup.rs) | IPC、权限及备份编排 |

实现参考 Termora `2.x` 的[统一接口思路](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/sync/src/main/kotlin/app/termora/plugins/sync/Syncer.kt)；其[插件许可](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/LICENSE)另有限制。本项目按 [GitHub REST](https://docs.github.com/en/rest/gists/gists)、[RFC4918](https://www.rfc-editor.org/rfc/rfc4918) 与 [SQLite serialize](https://www.sqlite.org/c3ref/serialize.html) 独立实现，未复制/翻译插件代码，也不兼容其备份格式。

# 数据备份与云端恢复

更新：2026-10-17。手动 Gist / WebDAV 备份、恢复预览、非覆盖合并和首次初始化云端恢复已有源码；仅完成静态检查，尚未编译、执行 Rust 回归或进行云端联调。数据库仍为加密格式2、schema 6，没有修改或新增业务迁移。

## 1. 范围与操作语义

设置窗口的“数据备份”页配置一个当前目标，支持 GitHub Secret Gist 或 WebDAV。用户手动选择“备份·上传新版本”或“同步·从云端下载”。上传每次创建独立对象，不修改既有备份；下载先解密展示时间、设备、应用版本和数量，再由用户确认新增合并。本功能不按远端时间自动选择覆盖，也不启动导入的终端、转发或工作区。

| 内容 | 是否包含 | 处理方式 |
| --- | --- | --- |
| SSH / Telnet / Serial 主机、分组、标签、收藏 | 包含 | 完整字段和关联校验；导入生成新的主机及分组 ID |
| 保存的工作区、端口转发规则 | 包含 | 重写主机引用与布局叶节点；不保存运行状态 |
| 公共与主机专属凭据、已导入私钥正文及口令 | 包含 | 仅在 Rust 内参与快照及加密，不经过前端 |
| SSH 主机信任 | 包含 | 同端点同公钥跳过；不同公钥保留本机记录并返回警告 |
| 应用设置、快捷键、自定义配色、连接历史 | 排除 | 恢复不改变本机内容；主机配色映射属于本机设置，不随快照导出 |
| 私钥原文件路径登记、OS 主密钥、vault.json | 排除 | 导入使用目标设备自己的加密保险库；不依赖原私钥文件 |
| 云端连接配置、Token / WebDAV 密码、备份密码 | 排除 | 云端账户在目标设备单独输入；首次恢复成功将此次输入的云端配置写入本机 |
| OTP、终端输出、打开的标签、广播成员、SFTP 正文及任务 | 排除 | 不属于持久化配置备份 |

这是一份包含全部允许类别的数据库快照，使用表白名单排除设置。它不是活动 SQLCipher 文件的物理复制，也不包含设置表的空页、删除残留或 SQLx 迁移记录。自动备份、自动保留清理、后台双向同步、逐项选择恢复、本地备份文件入口和显式取消 IPC 尚未实现。

## 2. 快照数据库与恢复

`terminal-storage::backup` 在同一个 SQLx 读事务中读取以下九表：`host_groups`、`connection_profiles`、`credentials`、`ssh_profiles`、`profile_auth_steps`、`tags`、`profile_tags`、`port_forwards`、`known_hosts`。`terminal-security` 的凭据操作门闩协调导出与凭据多步写入、改密，仅导出 ready 凭据，移除进行中的系统识别令牌。事务释放后才执行压缩、密码派生和网络操作。

`backup_sqlite` 在独立内存连接创建固定普通表，通过 SQLite serialize 得到真实 SQLite 数据库字节。快照使用独立 `user_version=1`，与业务 schema 6 分开演进；明文数据库不写临时文件。大小上限128 MiB，所有表合计最多100000行。全量处理仍需要内存，压缩率取决于内容，不能保证所有数据库都能压到云端限额内。

恢复不执行备份携带的 SQL。解析器校验 SQLite 文件头、版本、固定表结构及所有业务类型、字段和关联，拒绝额外表、视图、触发器、虚拟表及不匹配 schema；连接使用只读 deserialize、defensive、关闭 trusted schema/扩展/视图/触发器、只读 authorizer 与指令预算。不能因为 AES 认证成功就信任数据库内容。

所有写入在一个目标数据库事务内完成：

- 主机、分组、凭据、认证步骤、转发生成新 ID，重写关联；工作区引用一并重写。来源工作区已失效的引用保持失效，不误绑定目标库同 ID 主机。
- 主机名称冲突追加 ` (同步 n)`，标签按名称复用；既有主机、凭据和转发不被更新或删除。
- 信任记录核验算法、公钥和 SHA256 指纹；已有同端点同公钥跳过，密钥冲突跳过并警告，不自动改变信任。
- 普通恢复不写本机设置。首次云端初始化仅将此次输入的 `backup` 配置与业务导入在同一事务提交。

每次确认使用所属窗口绑定、有效期5分钟的一次性随机令牌；每窗口最多一份待确认明文快照，销毁窗口或到期清理。成功摘要短期缓存用于同进程、同令牌的应答重试，不提供跨进程持久幂等账本。用户重新选择同一备份再次导入，会新增副本；这不是按业务身份更新的双向同步。

## 3. 包格式与密码

外层为标准 Base64 文本：

```text
Base64(
  "TMETABK1" [8 bytes] + salt [16 bytes] + nonce [12 bytes]
  + AES-256-GCM(
      metadataLength [u32 big-endian]
      + metadata JSON
      + databaseLength [u64 big-endian]
      + Zstd(SQLite snapshot),
      AAD = magic + salt + nonce
    )
)
```

JSON 元数据仅在密文内，字段为备份 UUID、毫秒 Unix 时间、设备名称、应用版本以及主机/分组/凭据/转发/信任数量。压缩使用 Zstd level 3，先压缩完整快照，再将元数据和压缩结果一起加密；外层只包含格式和密码学参数。云提供方仍能观察对象大小、请求和平台自身的创建时间，应用不在文件名或描述主动暴露用户元数据。

独立备份密码通过固定 Argon2id v19（64 MiB、3轮、1并行度）派生32字节密钥，每包随机 salt 和 nonce，头部绑定为 GCM AAD。密码至少12字符、至多1024 UTF-8字节，不 trim，配置后与云端认证一起加密保存在本机 SQLCipher 设置中。云端 Token / 密码、本机主密钥均不作为备份密码。更改本机保险库密码或云端认证不改变旧包密码；修改备份密码只影响新包，读取旧包时需将配置切回旧密码。新设备仍需输入原备份密码，本机设置不会随云包导出。

解密先校验认证，再读取有界元数据、原始数据库长度和 Zstd 帧；限制解压窗口及输出到128 MiB，验证实际长度、数据库结构及元数据数量。密码、密钥和应用控制的明文缓冲使用 Zeroizing；第三方库内部复制和分配不等同于可保证的全内存安全擦除。安全模块格式解析最大192 MiB，实际云端入口采用下述更小限额，且流式读取不只依赖 Content-Length。

由于全部元数据都在密文内，列表必须下载并解密候选包才能展示备份时间和设备。默认每次10份、最多20份，支持继续加载；错误包显示读取失败，不让某个损坏包阻止其他备份展示。

## 4. 存储提供方与扩展

统一 `BackupProvider` 边界仅接触不透明加密包：`test`、`list`、`upload`、`download`。工厂根据 `BackupProviderKind` 创建实现；格式/加密、业务合并和 UI 不放入提供方；配置中的备份密码不属于提供方 target。后续增加 OSS 时新增适配器、配置类型与校验、工厂分支和界面字段，不改快照格式或导入事务。当前配置是 Gist/WebDAV 的具体字段；OSS 签名和认证 DTO 需随适配器扩展，不声称已经支持任意厂商。

HTTP 使用 reqwest，固定 HTTPS、系统证书验证、禁止重定向和自动重试，连接15秒、单次读取30秒、单请求总计120秒。整个分页/逐包列表操作可能包含多个请求。连接测试只读，不创建目录或占位文件；上传完成后 GET 回读比较完整加密包。若创建成功后核验失败或请求超时，提示先刷新检查，避免自动重试产生新备份。

### GitHub Secret Gist

使用 GitHub 官方 REST API 和用户 Token，每次 `POST /gists` 创建 `public:false` 的独立 Gist，文件名固定 `termeta-backup.txt`，描述不含设备、时间或主机信息。只读取本账户候选备份，分页最多扫描10000个 Gist、最多1000份匹配备份。截断文件仅从官方返回的 HTTPS `gist.githubusercontent.com` 读取原始正文，不携带 Token，不跟随重定向。包上限8 MiB；Secret Gist 不是访问控制保险箱，数据保密依赖整包加密。

### WebDAV

使用用户提前创建的 HTTPS 目录和 Basic 认证。测试 `PROPFIND Depth:0`，列表 `PROPFIND Depth:1`；XML 验证 DAV 命名空间、根目录集合、层级及字段限额，拒绝 DTD/未知实体和跨来源/越界 href。目录响应最多4 MiB、扫描10000项、最多1000份备份。

新对象名为 `termeta-<随机 UUID>.backup`，使用 `PUT If-None-Match:*` 并要求201新建结果，不允许降级成覆盖。仅接受规范对象 ID，不能通过下载参数读取其他路径。包上限32 MiB；服务端必须支持条件 PUT。

## 5. 设置、初始化与权限

云配置独立保存于本机 SQLCipher 的 `settings` 表 `key="backup"`、`value_version=2`，JSON 为 `{target: {provider, endpoint, username, secret, deviceName}, backupPassword}`；普通 `AppSettings` 仍在 `app` 键，恢复不会替换它。读取配置只返回 `hasSecret` / `hasBackupPassword` 标记，不回传认证秘密或已保存的备份密码。前端省略秘密时，后端仅在提供方、地址和用户名均未改变时复用已保存值。上传、列表和下载命令从设置读取密码，不再接受单独的密码参数；首次初始化在配置草稿里输入一次，确认导入时连同目标配置保存。备份密码不写 OS 凭据库，也不传入网络提供方。

旧 `value_version=1` 配置仍可读取并保留原认证信息，界面提示补填备份密码，保存后写版本2；不新增 SQL 迁移、不修改 schema 6。留空表示沿用已有密码，输入新值表示替换，并校验确认字段；设置关闭不隐式保存草稿。

首次密码页可选择本地新建或云端恢复：输入云配置和备份密码，加载列表、选包并解密预览，确认后使用另行输入的本机密码创建本机保险库，导入业务并保存云配置及备份密码。只有完全没有保险库的 setup 状态允许未解锁访问这些只读云操作；既有锁定保险库必须先解锁。初始化和本地创建共用互斥门闩，防止两个入口并发创建。

创建本机保险库与导入不是跨文件的原子事务：若创建成功而导入失败，保留加密数据库和配套元数据，界面刷新保险库状态，用户可解锁后重新配置同步；不得为自动重试删除保险库或丢弃已生成的加密材料。业务数据与云配置仍由单个数据库事务保证同时提交或回滚。

八个命令统一经 desktop IPC 门面、AppManifest 名单、权限组及调用窗口校验。main/settings 可操作备份配置与手动备份恢复；仅 main 获得云端初始化提交权限，不扩大通用会话 owner。数据库正文、已保存凭据和备份包正文不经过前端。

备份有独立单操作信号量，忙碌时直接提示而不排队积累数据；短时内存表锁不跨网络或数据库 await。确认后的导入由独立后端任务完成，窗口消失不取消已提交事务。界面显示忙碌状态，当前没有逐字节进度或主动取消命令。

## 6. 模块映射

| 位置 | 职责 |
| --- | --- |
| `crates/terminal-storage/src/backup.rs` / `backup_sqlite.rs` | 白名单数据库、结构与业务校验、ID重写、单事务非覆盖导入 |
| `crates/terminal-security/src/backup.rs` | Zstd、Argon2id、AES-GCM、Base64 与保险库操作协调 |
| `src-tauri/src/backup/` | 提供方 trait/工厂、Gist、WebDAV 和有界网络读取 |
| `src-tauri/src/backup_runtime.rs` | 本地配置、并发预算、所属窗口/到期令牌与完成摘要 |
| `src-tauri/src/commands/backup.rs` | 八个 IPC、权限、初始化和前后端编排 |
| `src/services/desktop/` | Rust/TS对应类型、命令及恢复事件 |
| `src/modules/backup/` | 共享表单、手动上传/下载、列表、确认预览及恢复后刷新 |
| `src/components/VaultGate.vue` | 首次云端恢复与本机密码输入 |

## 7. 参考与验证边界

参考 Termora 默认 `2.x` 分支 commit `f0acbb40845c4ab2ddeb588951204ee9a9f727a9` 的[统一接口](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/sync/src/main/kotlin/app/termora/plugins/sync/Syncer.kt)、[Gist](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/sync/src/main/kotlin/app/termora/plugins/sync/GitHubSyncer.kt)与[WebDAV](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/sync/src/main/kotlin/app/termora/plugins/sync/WebDAVSyncer.kt)实现思路。[插件许可证](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/LICENSE)另有限制；本项目按 [GitHub REST](https://docs.github.com/en/rest/gists/gists)、[RFC4918](https://www.rfc-editor.org/rfc/rfc4918)和[SQLite序列化 API](https://www.sqlite.org/c3ref/serialize.html)独立实现，没有复制或翻译其插件代码，也不兼容其备份格式。

已补充白名单/设置保留/关联重写、信任冲突、损坏结构、加密随机性/错误密码/篡改及提供方路径/XML边界的回归源码；未编译或执行。前端类型、结构、格式、Rust格式、ACL/迁移静态核对不能证明 SQLCipher 原生 API、三平台工具链或真实云服务可用。后续运行验收需覆盖跨设备往返、初始化中断、错误认证、权限/限流/超时、截断大包、条件PUT兼容性、恶意数据库、进程中断及本机设置/信任保留。

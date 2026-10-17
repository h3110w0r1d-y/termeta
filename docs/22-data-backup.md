# Gist / WebDAV 数据备份实现方案

日期：2026-10-17。本文为待实施方案，尚未实现备份命令、界面或远端请求，不代表需求完成或运行验收。现有加密格式2、schema 6、已应用迁移字节均不因本文改变。

## 1. 目标与交付范围

在独立设置窗口新增“数据备份”页，支持将同一种加密备份包保存到 GitHub Secret Gist、用户指定 WebDAV 目录或本地文件，并从中恢复。首版实现手动备份、连接测试、远端版本列表、下载、恢复预览与整体恢复；自动备份、自动保留策略、选择性合并后续实施。

备份是用户主动生成的时间点快照。每次上传创建独立对象，不在连接远端后自动拉取、合并、覆盖本地，也不把备份作为终端重连或自动执行入口。用户可分别配置 Gist 和 WebDAV，首版每类一个目标，手动选择本次目标。

### 备份内容

| 内容 | 首版默认 | 处理方式 |
| --- | --- | --- |
| SSH / Telnet / Serial 主机、分组、标签、收藏 | 包含 | 保留稳定 UUID、类型及完整配置，统一校验名称、树结构与关联 |
| 保存的工作区、端口转发规则 | 包含 | 保留主机引用，不保存活动连接、监听或广播状态 |
| 公共凭据、主机专属密码、已导入私钥及其口令 | 包含 | Rust 内部读取，仅进入加密正文；备份操作前明确展示包含凭据 |
| 应用设置、快捷键、自定义配色、主机配色引用 | 包含 | 使用独立版本化 DTO，复用当前设置验证；恢复后通知所有窗口重新读取 |
| SSH 主机信任记录 | 可选，默认关闭 | 单独展示数量和风险；未选择时保留目标设备当前信任库，不自动信任备份中的主机 |
| 最近连接历史 | 可选，默认关闭 | 活动记录归一为已中断，继续遵守20条限制 |
| 私钥原始文件路径登记、系统钥匙串、缓存与临时文件 | 排除 | 私钥正文已由 credentials 保存，跨设备不依赖原文件；串口配置内的设备路径保留但提示可能需重新选择 |
| Gist Token、WebDAV 密码、备份加密材料、备份任务状态 | 排除 | 目标设备单独配置，恢复不能替换本机备份账户或触发自动上传 |
| 终端输出、OTP、打开的标签、SFTP 文件正文及任务 | 排除 | 均不是此次持久化配置备份的范围 |

首版固定恢复全部基础类别，避免遗漏凭据或工作区依赖；仅信任、历史为独立选项。恢复预览明确区分“将替换的类别”和“保留本机的类别”。

## 2. Termora 参考结论

调研版本：Termora 仓库默认分支 `2.x`，commit `f0acbb40845c4ab2ddeb588951204ee9a9f727a9`。以下链接固定到该提交，避免后续分支变动改变依据。

| 参考位置 | 已观察到的行为 | 对本项目的取舍 |
| --- | --- | --- |
| [Syncer](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/sync/src/main/kotlin/app/termora/plugins/sync/Syncer.kt)、[SyncConfig](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/sync/src/main/kotlin/app/termora/plugins/sync/SyncConfig.kt) | 统一 pull / push 接口、供应商与数据范围配置 | 借鉴统一接口；Termeta 使用快照的 list / upload / download，恢复另设明确操作 |
| [GitHubSyncer](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/sync/src/main/kotlin/app/termora/plugins/sync/GitHubSyncer.kt) | Bearer Token；首次 POST 创建非公开 Gist，后续 PATCH 同一 Gist | 采用官方 Gist REST API；首版每个快照单独创建 Secret Gist，避免共享文件的并发覆盖 |
| [WebDAVSyncer](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/sync/src/main/kotlin/app/termora/plugins/sync/WebDAVSyncer.kt) | Basic Auth、MKCOL、GET、PUT，写入固定 JSON 文件 | 采用相同标准协议，改为独立快照文件；读取与测试不能隐式创建文件 |
| [SyncManager](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/sync/src/main/kotlin/app/termora/plugins/sync/SyncManager.kt)、[SafetySyncer](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/sync/src/main/kotlin/app/termora/plugins/sync/SafetySyncer.kt) | 变更后10–30秒防抖、先拉后推、按记录时间处理融合与删除；字段级 AES-CBC，密钥与服务凭据关联 | 借鉴任务串行与后续防抖；采用独立备份密码和整包认证加密，不按设备时钟自动决定覆盖 |

[plugins/LICENSE](https://github.com/TermoraDev/termora/blob/f0acbb40845c4ab2ddeb588951204ee9a9f727a9/plugins/LICENSE) 对该目录另有限制，允许查看学习，限制复制、修改及衍生使用。实施时依据 GitHub / WebDAV 官方协议独立编写，不复制或翻译其插件代码。本方案不承诺与 Termora 备份格式互通。

## 3. 数据格式与加密

### 选择逻辑快照

云端使用版本化逻辑备份，不直接上传活动 SQLCipher 主文件。当前数据库使用 WAL，复制主文件可能遗漏已提交数据；物理恢复还必须处理原配套 vault.json、密钥、迁移版本和文件切换。

由 storage 在同一只读事务中导出内部 `BackupSnapshotV1`，显式选择允许的业务表及字段，保留跨表 ID。事务期间与凭据多步写入协调，避免导出 pending / deleting 中间态；完成快照后立即释放数据库事务，再进行编码、加密和网络上传。不能复用公共 list_hosts / list_credentials 拼接，因为公共 DTO 有意隐藏秘密，且多次读取不构成一致快照。

逻辑快照不携带 SQLx 迁移记录、OS 主密钥及本机 vault.json。恢复时把数据导入目标设备已解锁的加密数据库，继续使用目标设备当前保险库密码和元数据。原有物理备份的“数据库与 vault.json 配对保留”规则继续有效，逻辑备份是新增的独立格式。

### 包结构

使用 UTF-8 JSON 容器，扩展名 `.termeta-backup.json`，便于 Gist 保存；正文只保存 Base64 编码的密文。首版不压缩，减少依赖与解压边界。

```text
BackupEnvelopeV1
  magic: "termeta-backup"
  envelopeVersion: 1
  kdf: 固定标识、随机16字节盐
  cipher: "aes-256-gcm"、随机12字节 nonce
  ciphertext: Base64（加密的 BackupSnapshotV1，含认证标签）

BackupSnapshotV1（仅加密正文内）
  snapshotVersion / appVersion / createdAt / backupId
  sourceVaultId / sourceDeviceId / includedCategories
  groups / profiles / authSteps / credentials / tags / profileTags
  workspaces（由 profile 类型承载）/ forwards / settings
  knownHosts? / history?
```

独立备份密码通过现有 Argon2id v19 派生32字节密钥，沿用固定64 MiB、3轮、1并行度；每包新盐、新 nonce。固定编码的包头绑定为 AES-GCM AAD，防止替换算法、盐或版本；解析器拒绝重复 JSON 键和未知算法。包内不能指定任意 KDF 资源预算。密码至少12字符、至多1024 UTF-8字节，不 trim，首设要求确认。

不使用 Gist Token、WebDAV 密码或本机 OS 主密钥作为备份密钥。更换云端凭据或本地保险库密码不会使已有备份失效；已有备份仍需备份时的独立密码。首版每次操作输入备份密码，不持久化；密码丢失无法恢复。未来自动备份必须另行明确用户授权的密钥持久化机制，不能假设自动解锁后能取得原保险库密码。

首版建议限制：明文快照最大16 MiB、总记录最多100000条、JSON 深度最多32层；WebDAV / 本地包最大32 MiB，Gist 包最大8 MiB。所有大小均在流式读取时检查，不能只相信 Content-Length；Base64 解码、集合数量及各字段继续独立限额。超出 Gist 限额时提示使用 WebDAV / 本地，不静默遗漏凭据或其他数据。

## 4. 后端与前端分工

| 位置 | 拟新增职责 |
| --- | --- |
| `crates/terminal-storage/src/backup.rs` | 内部快照 DTO、一致读事务、导入事务、关联与字段验证；不向前端提供秘密 |
| `crates/terminal-security/src/backup.rs` | 密码派生、认证加解密、受控敏感缓冲、备份与改密/凭据操作协调 |
| `src-tauri/src/backup/` | 任务服务、Gist / WebDAV 适配器、下载限额、目标配置、原生文件选择、取消与状态 |
| `src-tauri/src/commands/backup.rs` | DTO、调用窗口与任务所有权检查；不实现密码学或跨表 SQL |
| `src/services/desktop/` | 统一 typed IPC 门面和 Rust / TS 对应类型 |
| `src/modules/settings/BackupSettings.vue` | 目标配置、版本列表、进度、恢复预览；Naive UI 组件与共享响应式表单 |

首版不增加空领域 crate。网络在 Rust 中完成，Vue 只保留配置草稿、远端摘要和任务状态，不经过前端传递明文快照、保存的凭据或整包正文。复用现有 argon2 / aes-gcm / zeroize / Tokio；新增 HTTP 客户端及受限 XML 解析依赖时再核对固定 release 的 API、features、MSRV、许可证和安全公告，更新锁文件及预研。

远端目标与任务摘要使用加密数据库中的专用记录，与普通 `AppSettings`、SSH credentials 类型分离。若新增表，追加迁移，绝不改写0001–0005或 `_sqlx_migrations`。Token / 密码只允许写入替换或显式清除，读取仅返回 `hasSecret`；不保存到前端设置 store 或 OS 的额外凭据项。

所有备份任务使用独立串行队列和取消信号；数据库事务、凭据操作门闩与网络等待分离。快照期间可继续使用终端；改密与恢复不能和快照交错。进度区分读取、加密、上传、远端核验、完成；失败不更新最后成功时间。

## 5. Gist 适配

配置 GitHub Token；应用生成备份集标识。可列出当前账户的 Termeta 备份集，也允许输入已有备份 Gist ID 直接下载，方便新设备找回。首版固定 GitHub 官方 API 主机，不支持任意 API 镜像。

- 用 `POST /gists` 为每次快照创建 `public:false` 的 Secret Gist，包含一个固定命名的备份包；描述只含产品标识、随机备份集 ID 和必要版本标识，不含主机名、地址或用户名。
- 列表使用认证的 `GET /gists` 并完整处理分页；描述和文件名只是候选筛选，不能替代包格式与认证校验。不依赖“列表第一项”等同最新可恢复备份。
- 下载使用 `GET /gists/{id}`；检查整体和文件的 `truncated`。文件截断时才读取官方返回的 `raw_url`，严格限制 HTTPS / 官方 raw 主机与重定向，不能把 Authorization 转发给其他来源。
- 官方 API 的内联文件内容有约1 MB限制；这是响应截断边界，不能解释为 Gist 文件绝对只能1 MB。首版本项目限制包不超过8 MiB，超限切换目标，不引入 Git clone 依赖。
- 上传返回后重新读取并核对完整包摘要，成功才记录快照。创建请求超时可能已经成功，状态显示“结果待核验”；先按本次 backupId 查找，不能盲目重复创建或自动清理旧版本。
- Secret Gist 持链接可访问，隐私保护来自包加密。首版仅追加、不自动删除；以后清理只处理明确属于本机备份流的对象，不把 GitHub 内部历史承诺为可控保留或安全擦除。

Token 支持细粒度 PAT 的 Gists 读写权限；经典 PAT 使用 `gist` scope。连接测试仅验证读取身份/备份列表，不通过创建临时 Gist测试写权限；界面明确“读取连接通过，写权限在首次备份确认”。不申请无关 repo 权限。

## 6. WebDAV 适配

配置 HTTPS 目录 URL、用户名、密码或应用专用密码。允许用户指定内网 HTTPS 地址；证书错误须修正服务端或系统信任，不提供忽略校验开关。首版不支持明文 HTTP、URL 内嵌凭据、跨来源认证重定向。

使用 `MKCOL`、`PROPFIND Depth:1`、`PUT`、`GET`；后续保留清理才引入 `DELETE`。首版要求用户提供已存在的基础目录，首次上传在其下创建应用备份子目录。只在用户备份操作中创建目录，列表/测试404不创建空文件。连接测试是有界 PROPFIND，仅证明读取能力。

路径示例：`<用户目录>/termeta/<备份集UUID>/<时间戳>-<backupId>.termeta-backup.json`。名称随机且永不复用，PUT 携带 `If-None-Match:*` 防覆盖。首版不依赖 MOVE、共享 latest.json 或远端锁：服务端中断可能留下不完整的新文件，但不会替换旧快照；列表只代表候选，下载完整校验通过后才可恢复。上传后 GET 核验摘要，通过才标成功。

PROPFIND 解析限制响应体、元素数与 href 数量，禁用外部实体；href 必须属于原来源与配置目录，拒绝目录越界、绝对外域和路径穿越。时间排序不是信任凭据。区分401/403、404、405、412、配额不足、证书、超时及网络取消；服务不支持必要方法时直接说明不兼容，不能退回普通无保护覆盖。

## 7. 恢复流程与一致性

1. 用户选择快照并输入备份密码；后端限额下载，校验外层格式，派生密钥并验证 AEAD，再解析内部版本与全量业务结构。密码错误/篡改不能进入数据库写入。
2. 验证分组无环、四类名称 trim 后区分大小写且唯一、凭据种类/归属/状态、配置字段、配色与快捷键、转发和工作区引用。备份中原本已失效的工作区或转发引用保留失效语义并在预览提示，不能按名称重绑；新出现的异常关联拒绝导入。
3. 返回类别、数量、时间、来源及失效引用提示，生成有期限、绑定窗口/包摘要/本机数据代次的 `restoreToken`；前端不持有解密正文。显示“将替换本机这些类别”，二次明确确认。
4. 提交只由 main 执行。要求没有活动终端、SFTP、转发和认证任务，用户主动关闭后继续；后端重新检查并进入恢复维护状态，拒绝新建会话和并发业务写入。settings 只发出请求，由 main 展示最终确认，不扩大通用 owner。
5. 在本机独占私有目录先生成当前状态的加密回滚快照，使用本次已验证的备份密码及全新随机盐/nonce，绑定当前 vault UUID；回滚快照写入、同步并验证成功才继续。密码只在短期恢复任务内受控保留，取消/超时/完成后释放，不依赖从已解锁连接池或 OS 再次取得本机主密钥。空间不足或回滚快照失败保持原库。
6. 在当前 SQLCipher 数据库单事务中按依赖顺序替换选中业务类别、检查外键并提交。保留本机 vault 元数据、主密钥、备份目标及未选类别，不复制远端 SQLx 迁移记录。所有 SQL 均固定/参数化，不执行备份携带的 SQL。
7. 事务失败或提交前崩溃依靠数据库回滚；提交成功则整份新数据生效。写入本地恢复完成标记与业务替换处于同一事务，处理提交结果不确定的重启恢复，不能凭 UI 超时重复导入。
8. 成功后递增本机数据代次，旧预览 token、排队写入和旧窗口草稿全部失效，通知 main/settings 重新加载；不得因恢复设置触发备份。数据代次校验必须覆盖跨窗口旧请求，不能只依赖前端通知或可能碰巧相同的 revision。保留回滚快照供显式恢复，不自动连接任何主机。

新设备先创建并解锁新的本地保险库，再配置远端或导入本地包；恢复不需要来源设备的保险库密码、OS 钥匙串或 vault.json。首版不提供锁定态云备份命令，保持现有未解锁仅允许 status/unlock 的边界。原库损坏的抢救沿用既有数据保留规则，不能通过“恢复”按钮先删库。

## 8. 拟新增 IPC 与界面

以下为待实施命令族，实际落地须同步 Rust / TS 类型、AppManifest::commands、handler、capability 与所有权检查，不复用通用 export_json 传递秘密：

| 命令族 | 窗口与用途 |
| --- | --- |
| `get_backup_targets` / `save_backup_target` | main/settings，脱敏摘要与写入配置；秘密字段省略保留、显式清除与替换语义一致 |
| `test_backup_target` / `list_backups` | main/settings，只读连接验证和分页摘要 |
| `start_backup` / `get_backup_job` / `cancel_backup_job` | main/settings，owner-bound 任务；共享目标的同类任务串行 |
| `export_backup` / `import_backup` | main/settings，后端原生选择器绑定路径，不接受普通 IPC 任意路径 |
| `prepare_backup_restore` | main/settings，后端验证后只返回摘要及短期 token |
| `request_backup_restore` / `commit_backup_restore` | settings 向 main 请求最终确认；提交仅 main，后端重新验证任务与状态 |

任务采用轻量进度事件和可重读状态，不能用事件正文传包内容或凭据。关闭窗口取消其未提交网络/准备任务；恢复进入数据库提交阶段后由后端完成，UI 阻止关闭，不把窗口关闭等同事务取消。取消上传结果也可能待核验，不能误报远端不存在。

设置页配置草稿与普通设置自动保存队列分离：连接参数合法时可保存，密码只写不读；备份、恢复和连接测试均为显式动作。恢复预览与确认放 AppDialog footer，NForm model/rules/path 共用响应式字段，图标继续使用 Ionicons + NIcon。

## 9. 实施顺序与验收门槛

| 阶段 | 具体交付 | 退出门槛 |
| --- | --- | --- |
| A 本地闭环 | 快照 DTO、加解密、本地导出/导入、预览与事务恢复、回滚快照 | 新保险库完整恢复主机/凭据/工作区/转发/设置；错误密码/损坏零写入；旧草稿不覆盖恢复结果 |
| B 云端适配 | Gist、WebDAV、目标秘密存储、版本列表、任务取消与上传核验 | 分页/截断/超时不确定状态可处理；云端异常不损坏本地；新设备可定位和恢复 |
| C 可选增强 | 自动备份密钥保管、变更防抖、周期补偿、保留清理、选择性合并 | 自动功能默认关闭；只在解锁后运行；失败退避；恢复不反向触发上传；多设备清理边界明确 |

阶段A/B构成首版范围。后续自动备份监听持久化配置变化，排除终端输出、连接历史高频变化及备份任务自身写入；恢复永远是手动动作。选择性合并需另定冲突与删除语义，不采用设备时间直接覆盖。

需要编写并在用户授权后执行的验证：快照往返与全部关联、秘密不进入公共 DTO/日志/磁盘明文、加密随机性与篡改/错误密码、限制与畸形格式、同事务一致性、凭据变更/改密互斥、恢复提交中断和代次、磁盘满、目标配置旧草稿、Gist 分页/truncated/限流/创建结果不明、WebDAV 207 XML/href/方法缺失/配额/部分上传、证书与认证重定向、窗口 ACL 与三平台文件权限。HTTP 测试使用受控模拟服务，不连接真实用户账户或设备。

本轮仅阅读本仓库与参考源码、核对公开文档并编写方案；没有新增运行依赖、修改业务代码、操作用户数据库/钥匙串、创建远端对象、编译或启动项目。

## 10. 官方协议依据

- [GitHub Gists REST API](https://docs.github.com/en/rest/gists/gists)：Secret Gist、创建/读取、分页、细粒度权限与 truncated/raw_url 边界。
- [GitHub Secret Gist 可见性](https://docs.github.com/en/get-started/writing-on-github/editing-and-sharing-content-with-gists/creating-gists)：Secret Gist 仍可通过链接访问。
- [RFC 4918：WebDAV](https://www.rfc-editor.org/rfc/rfc4918)：MKCOL、PROPFIND、207 Multi-Status 等协议语义。
- [RFC 9110：HTTP 条件请求](https://www.rfc-editor.org/rfc/rfc9110#section-13)：If-None-Match 与状态处理。
- 本项目现状与边界：[存储](07-storage.md)、[安全](08-security.md)、[路线](10-roadmap.md)、[IPC](06-ipc-and-streaming.md)。

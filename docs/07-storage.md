# 加密数据保险库与持久化

更新：2026-10-17。当前为加密格式2、schema 6，接受同加密格式 schema 2–5 原位升级，不迁移早期明文或旧凭据格式。SQLCipher 真实升级、OS 钥匙串与三平台集成尚未完整验收。

## 1. 存储边界

| 位置                               | 内容                                                                                                    | 边界                                                                                         |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `termeta-encrypted-v2.sqlite3`    | SQLCipher 4 加密的全部业务表：主机、分组、标签、密码、私钥及口令、信任、设置、历史、导入来源路径        | AES-256-CBC 页面加密、逐页 HMAC-SHA512；WAL 同样加密，临时存储仅内存                         |
| `termeta-encrypted-v2.vault.json` | 格式2、保险库UUID、Argon2id参数标识、16字节随机盐、12字节随机nonce、AES-256-GCM校验记录、初始化完成标记 | 没有密码或主密钥；须与数据库一起备份；AAD绑定UUID、参数、盐和完成标记；标记变更重新随机nonce |
| OS 凭据库                          | 唯一32字节主密钥                                                                                        | service=`io.termeta`，account=`master-key-v2`；不再逐项保存密码或私钥           |
| Rust 内存                          | 已解锁连接池、短期密码与认证材料                                                                        | 不返回主密钥或保存的秘密，不承诺进程内所有第三方副本可擦除                                   |

SQLCipher由锁定的libsqlite3-sys 0.30.1内嵌4.5.7源码和vendored OpenSSL提供；不是普通SQLite上的自制字段加密。`cipher_version`必须是4.x，否则拒绝初始化；不回退明文。SQLx先应用raw key再访问页面，禁用数据库语句日志。Unix目录0700，新DB与元数据0600；Windows权限继承待验收。

## 2. 首次设置与解锁

首次无数据库和元数据时，显示设置/确认密码，至少12个字符、最多1024 UTF-8字节。不trim或变换密码。Rust在阻塞工作池使用Argon2id v19（64MiB、3轮、1并行度）输出32字节主密钥，参数固定受版本约束，不接受文件指定任意内存预算。

先原子发布含随机盐与认证校验记录的元数据，再创建加密DB及schema 6，最后原子标记初始化完成。元数据通过独占临时文件、同步、原子发布持久化；中断初始化可用原密码重试。只有元数据完成并且数据库成功解密、完整性核验及结构初始化后，才向业务命令开放保险库。

随后将主密钥保存到系统钥匙串。如果保存失败，本次仍保持已解锁，界面明确提示下次需输入密码；不删除数据库，也不退回明文。

后续启动每进程最多自动读取一次OS主密钥：先通过GCM校验，再检查SQLCipher页面/逻辑完整性、结构版本；成功才加载页面和配置。钥匙串缺失、拒绝、错误密钥或解密失败时显示密码入口。密码派生和校验成功后打开原库，再覆盖保存唯一OS主密钥。错误密码不修改数据库、元数据或OS项。数据库损坏/版本不支持不视为首次启动，不重建空库。完成态元数据对应DB缺失或截断、DB存在但元数据缺失均报错。

仅`vault_status`与`unlock_vault`可在未解锁时使用，仍校验main窗口与本地导航；其余用例命令返回`VAULT_LOCKED`，不启动SSH/SFTP。数据库仓库也独立拒绝未解锁访问。关闭窗口仍可用；退出后需重新走解锁。已解锁后可修改保险库密码；没有遗忘密码重设、自动锁定或独立密钥轮换入口。

### 修改密码与中断恢复

设置“安全”页提供旧密码、新密码与确认密码。已自动解锁也必须重新验证旧密码；新密码遵守首次设置的字符/字节限制，不做 trim，且不能与旧密码相同。Argon2id 在阻塞池以新的随机盐派生密钥，保留保险库 UUID，使用新 nonce 生成校验记录；加密格式2与 schema 6 不变。

[改密协调器](../crates/terminal-security/src/master_key.rs)串行协调启动/改密与凭据多步写入；[存储切换](../crates/terminal-storage/src/password_change.rs)以数据库专用生命周期读写门闩暂停新查询，并关闭原池等待事务归还。切至 DELETE 日志模式并关闭连接后复制完整加密主文件，只在独占副本上使用 SQLCipher raw-key `PRAGMA rekey`；用新连接验证新密钥、页面/逻辑完整性及外键后才切换。全程不生成明文数据库，不改写迁移记录。重开业务池仅校验，不重复运行启动迁移或将活动历史标记为中断；终端输出/取消不经过该门闩。

同目录 `termeta-encrypted-v2.password-change/` 持有旧加密数据库/元数据副本、新加密副本及切换标记。Unix 目录0700、文件0600；写入同步，标记以临时文件加重命名发布，目录变更同步。只有 `prepared` 持久化后才可替换正式文件；新文件组合重新打开并验证后发布 `committed`。启动读取 vault.json 前恢复：未提交恢复旧组合，已提交保留新组合；恢复过程自身可重复执行，原始/被替换副本保留到恢复后的数据库通过密钥、完整性及迁移校验，校验失败不清理副本。`rolled-back` 防止清理失败后的下次启动再次覆盖恢复后的业务写入；清理先持久化删除 prepared，再删除终态标记。记录损坏或恢复失败时保留全部文件并拒绝开放业务，不重建空库。Windows 沿用文件同步及可重试的逐文件重命名，目录持久性和三平台故障矩阵仍待实测，不宣称覆盖所有断电/存储设备故障。

新组合提交后才覆盖 OS 的唯一主密钥。钥匙串失败不回滚已完成的改密，返回成功及“下次输入新密码”警告；旧加密副本清理失败也返回成功及单独警告，下次启动在校验数据库成功后重试，仍失败则解锁结果继续提示清理警告。改密需要额外磁盘空间，空间不足应保护原文件。若发现 password-change 目录，应与数据库、vault.json、WAL/SHM 一起保留，不能只删除恢复记录。旧备份及清理前的旧副本仍可用旧密码解密，不承诺从 SSD/快照中安全擦除旧数据。

## 3. 保存项、分组与凭据

`connection_profiles` 保存 SSH、Telnet、Serial、Workspace；历史约束另有 local，但当前本地终端不创建保存配置。公共 HostProfile 返回轻量元数据和可选 workspace/serialConfig，HostDraft 不含秘密，缺失 kind 默认 ssh。保存连接类型不可就地转换。

| 类型      | 参数与关联                                         | 约束                                                                                      |
| --------- | -------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| SSH       | ssh_profiles、认证引用及 config_json 图标/检测状态 | 凭据由 Rust 加载；公共 DTO 不返回秘密或主机专属 credentialId                              |
| Telnet    | config_json.telnetConfig 的地址/端口               | username 空、authPlan 空，无 serialConfig；拒绝 SSH 密码/认证引用                         |
| Serial    | config_json.serialConfig 全部参数                  | address/username 空、port=0、authPlan 空；拒绝密码；保存只验证语法，不访问设备            |
| Workspace | config_json.workspace 的 SavedWorkspace            | 无 SSH 子表/认证/秘密；只允许 save_workspace 创建/整体替换，save_host 仅允许分组/收藏变更 |

密码/私钥及可选解密密码保存在加密 credentials.secret，不依赖私钥原文件；公共凭据列表只返回 id/kind/label/status，主机专属项不列出。主机直接密码非空替换、显式空串移除、省略/null 保留；复制主机不复制专属密码。引用公共凭据须类型匹配且 ready，专属项须属于对应主机。

凭据准备为 pending，secret 与 ready 同次写入；缺失秘密的 pending 项删除后重建。替换在 revision/归属校验事务中绑定新项并将旧项登记 deleting；失败保留原引用，提交后的清理失败不伪报保存失败，解锁后定向回收。被引用公共项禁止删除，不扫描或删除早期 OS 凭据。

分组父级变更防环并校验 revision。默认删除将直属主机/子组上移；deleteHosts=true 在 BEGIN IMMEDIATE 中递归删除子树与主机、登记专属凭据清理，整体失败回滚。公共凭据/信任和已有会话快照保留，历史 profile 外键置空。主机删除后转发引用置空，工作区 JSON 中的 ID 引用有意保留失效状态。

SSH 的 `config_json.startup` 保存 `{type:"shell"}` 或 `{type:"exec",command}`，HostDraft/HostProfile 与运行 SshConfig 使用同一 SshStartup 类型；保存、读取和连接时校验，旧记录缺失字段默认为 Shell，显式 null/未知类型/无效命令拒绝。命令原文在现有 SQLCipher 数据库内持久化；分组/收藏修改、主机复制保留启动方式，不复制专属密码。Telnet/串口/工作区禁止 exec 配置。

该字段不改变 schema 6 或已有迁移字节；工作区仍只存 profileId，下次打开/手动重新运行读取最新主机设置，不更新活动会话。旧版本应用可能忽略新字段，不保证降级后仍按命令模式连接。

## 4. 格式、备份与恢复边界

Gist / WebDAV 手动备份使用九表白名单的内存 SQLite 快照，先 Zstd 压缩，再将时间、设备名等元数据与数据库一起 AES-256-GCM 加密并 Base64 编码。仅包含主机/分组/标签/工作区、凭据/认证关联、转发及主机信任，排除 settings、history、key_files、SQLx迁移、来源主密钥和 vault.json。快照独立 user_version=1，不改变业务 schema 6 或已应用迁移。详见[数据备份](22-data-backup.md)。

上传创建新对象；恢复经过格式、表结构、业务关联与凭据校验后单事务新增合并。实体生成新 ID并重写工作区及凭据引用；同名主机另存，标签复用，已有信任冲突跳过并警告，既有本机数据和设置保留。首次初始化先创建目标保险库，再将业务数据与本次输入的云配置在同一事务提交；失败保留本机加密文件和派生元数据以便重试。备份128 MiB/100000行和解压限额、初始化中断边界及重复导入产生副本的语义见备份文档，尚未运行验收。下述物理备份配对保留规则继续有效。

当前路径不打开旧 `termeta.sqlite3` 或 `termeta-credentials-v1.sqlite3`。源码/锁文件更新不直接操作实际用户数据库和钥匙串。

无自动备份/恢复、自动锁定或独立密钥轮换入口。修改密码更换当前数据库密钥，既有数据库物理副本仍使用原保险库密码；云端备份使用独立备份密码。手工保留物理备份应在应用退出后保留整个应用数据目录，包括加密数据库、配套 vault.json 及可能存在的 WAL/SHM；不能把普通 VACUUM INTO 当作默认加密备份。密码与 OS 主密钥均丢失时无法恢复；缺失原随机盐/校验元数据时也不能按现有接口恢复。旧明文库不会自动加密。

## 5. 设置与连接历史

app 设置为带版本的 JSON，language 可选 system/zh-CN/en-US，旧值缺失按 system；该偏好随本机加密设置保存、不进入云备份，无需迁移或改变 schema 6。Rust 拒绝未知字段/非法值；main/settings 窗口共用设置专用串行队列，读取最新值后局部合并、校验、写入，成功后通知其他窗口重读。hostsView/hostSearch 按子字段合并，旧草稿不能覆盖即时偏好。无效值/写入失败保留此前持久值，设置表单保留输入并提供重试。

| 字段                         | 默认/校验                                                                                                                     |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| primaryColor                 | 缺失或空串沿用 Naive UI 当前明暗默认色；自定义只接受不透明六位 HEX                                                            |
| hostsView                    | includeChildren=false、expandedGroupIds=[]、showHosts=false、displayMode=card；展开键仅分组 UUID/ungrouped，去重且最多10000项 |
| hostSearch                   | pinyin=true，name/address/username/tags/group 全选；name 固定，拒绝重复/未知字段                                              |
| sftpShowHidden               | false，终端侧栏和双栏同步                                                                                                     |
| 字体、配色、快捷键、标签位置 | 白名单/版本/引用校验，旧字段按兼容规则归一，详见对应模块文档                                                                  |

普通 app 设置最大256 KiB。搜索查询与拼音索引只驻内存。

云目标独立使用 `settings.key=backup`、`value_version=2`，JSON为 `{target:{provider,endpoint,username,secret,deviceName},backupPassword}`，在本机 SQLCipher 内加密保存；get_settings 不读取或返回此键，专用 get_backup_config 只返回 hasSecret/hasBackupPassword 状态，不回传秘密。备份自身不含该设置；首次恢复保存的是用户此次输入的目标配置。普通同步不替换本机云账户；备份密码与Token一样加密保存在本机设置，操作时由Rust读取，网络提供方不接收密码。旧value_version=1保留原目标和认证，补填密码后保存为版本2，无业务数据库迁移。

session_history 合计保留 SSH/Telnet/Local/Serial 最近20次连接尝试，不按主机去重。新增与裁剪同一事务提交，解锁初始化时裁剪既有超量记录；按 started_at DESC、rowid DESC 读取最多20条。旧活动摘要被裁剪后，结束更新为空操作，不影响会话或终端缓冲。Local 的 profileId 为空、endpoint=local；独立 SFTP/转发不写终端历史。正文、环境和远端标题不入历史。

## 6. schema 与迁移

SQLCipher 密钥和页面完整性校验后，在尚未发布业务池的专用连接上检查 user_version，仅接受0或同格式2–6。SQLx Migrator::run_direct 在该连接运行迁移；父表重建前于事务外关闭 foreign_keys，迁移后恢复并执行 foreign_key_check，检查通过才发布业务池。迁移失败不开放业务，保持 SQLx 事务回滚与校验和检查。

| 迁移文件                    | 结果 schema | 内容                                                    |
| --------------------------- | ----------- | ------------------------------------------------------- |
| 0001_initial.sql            | 2           | 加密格式2的初始业务表、SSH/Local/Serial 类型约束        |
| 0002_telnet.sql             | 3           | 重建 connection_profiles 扩展 Telnet，保留父子关联和 ID |
| 0003_forwarding.sql         | 4           | 增加 port_forwards，主机外键 ON DELETE SET NULL         |
| 0004_workspaces.sql         | 5           | 增加 workspace、处理历史重名并创建名称唯一索引          |
| 0005_profile_name_index.sql | 6           | 将唯一索引更新为 trim(name)，其余业务记录和迁移历史保留 |

新库依次应用全部迁移，同格式旧库应用缺少的迁移；数据库文件名、主密钥和 vault 元数据不变。已应用迁移原始字节必须保持，后续改动只能新增迁移，不修改 `_sqlx_migrations`。升级后的库不能交回不识别该 schema 的旧应用，回退使用升级前一致备份。

## 7. 转发与系统识别状态

port_forwards 保存 id/name/profile_id/rule_json/revision/时间戳。保存/启动仅接受已保存 SSH 主机，启动重读最新连接/凭据；规则不复制秘密，不保存运行状态或自动启动。编辑/删除校验 revision，活动期间由命令拒绝修改；删除主机将引用置空但保留规则，已有独立连接可继续至停止。见 [转发设计](21-port-forwarding.md)。

SSH 的 icon、systemDetectionAttempted 和后端专用 systemDetectionToken 位于 config_json。表单不能回写标记/令牌。claim 以 BEGIN IMMEDIATE 同时读取 detectSshSystem 开关和登记一次性机会，关闭时不消耗；失败也不撤销标记。complete 仅按令牌/目标/默认图标条件更新内置 icon，不保存探测正文或复活已删除主机。claim/complete 更新 revision，旧草稿需重载；修改目标/图标作废令牌但不重置机会，复制保留 icon 而不继承标记。详见 [系统识别](05-ssh-design.md#10-一次性主机系统识别)。

## 8. 工作区存档与统一名称

四类保存项共享 trim(name) 的二进制唯一索引 profiles_unique_name，区分大小写。迁移保留旧重名中按 created_at/id 排序的首项，其余加无冲突序号、名称不超过128字符；重命名递增 revision，不改 ID、配置、凭据或引用，不合并/删除主机。

save_host 事务检查跨类型重名。save_workspace 新建需空 expectedRevision；同名替换的目标必须为 workspace 且 revision 匹配，保留 ID/分组/收藏，只更新布局/引用；并发新建或过期替换返回冲突。save_host 对工作区只允许受 revision 保护的分组/收藏变更。

SavedWorkspace 的叶子为保存 SSH/Telnet/串口 ID 字符串，非独立外键行；保存时事务验证全部引用，拒绝失效或嵌套工作区。删除主机保留失效存档，不按名称重绑。前端打开前验证整份引用，各 saved 会话入口再校验真实目标；不保存秘密、sessionId、输出或广播。见 [显式保存工作区](18-terminal-workspaces.md#8-显式保存与替换工作区)。

## 9. 已应用迁移校验和不一致的恢复

0004_workspaces.sql 保持首次实际应用的原始字节，SHA-384 为 `cee4f9328cb293cb70c2c2f2009d0dadcb0a80b73e843a2fad34e421f3801517fb71f65f81061409bf2871ed3ded0f29`。索引表达式更换仅在迁移5中执行，schema 5 可正常升级至6，SQLx 继续校验全部已应用迁移。

若遇 “migration 4 was previously applied but has been modified”，完全退出应用并备份整个应用数据目录，再由用户重新编译/启动包含当前迁移的版本；迁移编译期嵌入，旧二进制不会使用恢复后的 SQL。macOS 默认目录为 `~/Library/Application Support/io.termeta/`。不要删除数据库、vault 元数据、钥匙串、迁移记录或手工改校验和；仍不匹配时保留备份并核对实际应用过的版本。

## 10. 验证边界

已有锁定拒绝、密钥/篡改、初始化与钥匙串注入、凭据/分组事务、各类型往返/拒绝、统一名称/工作区并发、迁移摘要与同格式升级的回归源码，尚未编译执行。历史静态核对包括格式、原迁移摘要与内存 SQLite 模型，不代表 SQLCipher 真实升级或恢复已通过。已增加旧密码拒绝/新密码解锁、钥匙串失败、并发改密、WAL/迁移/活动历史保留、排队写入及逐文件切换/恢复中断的回归源码，均未编译执行。需授权后覆盖原生依赖、DB/WAL/临时文件加密、OS 权限/弹窗、初始化与改密中断、磁盘满、备份恢复及三平台矩阵，见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。

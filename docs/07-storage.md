# 加密数据保险库与持久化

更新：2026-10-28。本文维护本机持久化、升级与故障恢复契约；云包格式和导入见 [数据备份](22-data-backup.md)。当前为加密格式 2、schema 11，支持同格式 schema 2–10 原位升级，包括当前 cloud 分支的 schema 10 保险库；升级后不能交回仅识别 schema 10 的版本。不迁移早期明文库或旧凭据格式。

## 1. 存储边界

| 位置 | 内容与边界 |
| --- | --- |
| `termeta-encrypted-v2.sqlite3` | SQLCipher 4 加密业务表，包括主机、凭据、信任、设置、Snippets、会话日志与私钥来源登记；页面 AES-256-CBC、HMAC-SHA512，WAL 同样加密，临时存储仅内存 |
| `termeta-encrypted-v2.vault.json` | 格式、保险库 UUID、固定 KDF 标识、16 字节盐、12 字节 nonce、AES-256-GCM 校验记录和初始化标记；不含密码/主密钥，必须与数据库配对保留 |
| OS 凭据库 | 只缓存一个 32 字节主密钥：service=`io.termeta`、account=`master-key-v2` |
| Rust 内存 | 已解锁连接池及短期秘密；秘密不通过公共 DTO 返回，不能保证第三方库内部副本全部擦除 |

数据目录由 Tauri `app_data_dir()` 解析，Windows 为当前运行用户的 `%APPDATA%\io.termeta`，macOS 为 `~/Library/Application Support/io.termeta`，不随程序安装目录或安装范围迁移。macOS 主密钥缓存使用 Keychain；Windows 使用 `keyring 3.6.3` 的 `windows-native` 后端写入当前用户 Windows 凭据管理器的通用凭据，目标名为 `master-key-v2.io.termeta`。OS 库仅缓存主密钥，SSH 密码和私钥正文仍在加密数据库中。

Windows 所有用户安装只共享程序文件，不共享数据库或主密钥；同一 Windows 用户切换安装范围且应用标识不变时仍使用原数据与凭据项。以另一账号启动应用（包括使用另一管理员账号运行）会切换到该账号的数据目录和凭据库，不会自动读取原账号保险库。

SQLCipher 由锁定的 `libsqlite3-sys 0.37.0` 内嵌 4.10.0 源码（基于 SQLite 3.50.4）与 vendored OpenSSL 提供，SQLx 固定 0.9.0。连接先应用 raw key 再读页面，使用 SQLCipher 4 兼容参数并拒绝缺少 SQLCipher 4 的环境，不回退普通 SQLite；禁用 SQL 语句日志。Unix 应用目录 0700、数据库及元数据 0600；Windows 沿用当前用户数据目录的 ACL，不额外设置文件 ACL。

实现入口：[存储连接与初始化](../crates/terminal-storage/src/lib.rs)、[主密钥与元数据](../crates/terminal-security/src/master_key.rs)。

## 2. 首次设置与解锁

新密码至少 12 个字符、最多 1024 UTF-8 字节，不 trim。固定 Argon2id v19（64 MiB、3 轮、1 并行度）派生 32 字节主密钥；不接受元数据指定任意 KDF 预算。GCM 的 AAD 绑定保险库 UUID、KDF、盐及初始化标记，标记变化使用新 nonce。

首次初始化先原子发布元数据，再创建加密数据库及迁移，最后标记完成；中断可用原密码重试。只有密钥、页面/逻辑完整性、迁移及外键检查成功后才开放业务池。已完成的元数据对应数据库缺失/截断，或数据库存在而元数据缺失时均拒绝开放，不重建空库。

每进程最多自动读取一次 OS 主密钥，失败后由用户输入密码解锁。错误密码不修改数据库、元数据或 OS 项；成功后尝试保存唯一主密钥，缓存失败仅警告，本次仍已解锁。普通业务及仓库访问要求解锁；`vault_status`、`unlock_vault` 例外，仍校验本地 main 窗口。没有遗忘密码重设、自动锁定或独立密钥轮换入口。

### 修改密码与中断恢复

修改保险库密码要求已解锁并重新验证旧密码；新密码遵守首次设置限制且不能与旧密码相同。保留 UUID，生成新盐和 nonce，格式与 schema 不变。实现见 [主密钥协调](../crates/terminal-security/src/master_key.rs) 与 [文件切换](../crates/terminal-storage/src/password_change.rs)。

流程与凭据多步写入串行，数据库生命周期门闩暂停新查询并等待原池归还事务。关闭 WAL、切至 DELETE 日志模式后复制完整加密主文件，仅在独占副本执行 SQLCipher rekey；验证新密钥、完整性和外键后切换正式数据库及元数据。全程不生成明文库、不改迁移记录；重新开放业务池不重复启动迁移或改写会话日志，终端输出/取消不经此门闩。

同目录 `termeta-encrypted-v2.password-change/` 保存新旧加密文件和持久化标记：

- `prepared` 发布后才替换正式文件；新组合打开验证后发布 `committed`。
- 启动在读取 vault.json 前恢复：未提交恢复旧组合，已提交保留新组合。`rolled-back` 防止清理失败后的再次恢复覆盖新业务写入。
- 恢复可重复执行，副本保留到数据库通过密钥、完整性与迁移检查；记录损坏或恢复失败时保留全部材料并拒绝业务。
- 新组合提交后才更新 OS 缓存。缓存或旧副本清理失败返回成功附警告；清理在后续成功解锁后重试。

改密需要额外磁盘空间。故障时退出应用，配对保留数据库、vault.json、WAL/SHM 及整个 password-change 目录，不能只删恢复记录。旧备份/尚未清理的副本仍可用旧密码解密；不承诺从 SSD 或系统快照安全擦除。

## 3. 保存项、分组与凭据

`connection_profiles` 保存 SSH、Telnet、Serial、Workspace；本地终端不创建保存配置，类型不可就地转换。

| 类型 | 持久化契约 |
| --- | --- |
| SSH | `ssh_profiles`、认证引用与 `config_json`；Rust 加载凭据，公共 DTO 不返回秘密或主机专属 credentialId |
| Telnet | `config_json.telnetConfig` 地址/端口；无 SSH 用户名、认证或秘密 |
| Serial | `config_json.serialConfig`；无 SSH 地址、认证或秘密，保存不访问设备 |
| Workspace | `config_json.workspace` 布局和保存主机引用；只由 `save_workspace` 创建/整体替换 |

密码、私钥正文及可选解密口令保存在加密的 `credentials.secret`；私钥直接输入正文，既有凭据仍可使用，不依赖原文件。历史私钥来源登记保留兼容，新建凭据不再登记路径或文件引用，不改变 schema 或已应用迁移。公共列表仅返回公共项元数据。公共引用须类型匹配且 ready，专属凭据须属于对应主机。直接主机密码非空替换、显式空串移除、省略/null 保留；复制主机不复制专属密码。

公共密码与输入的私钥使用单条 INSERT 原子提交元数据、秘密及 ready 状态，失败不留下 pending 占位项。主机专属密码替换仍用 pending/ready/deleting 协调：先准备新项，再事务绑定新项并登记旧项 deleting；失败保留原引用，提交后清理失败不改变保存成功语义。解锁协调在专用操作锁内回收无认证引用的主机孤立项及历史公共 pending 项，保留可用公共 ready 项和仍被引用的记录。被引用公共项不能删除，不扫描旧 OS 凭据。实现见 [凭据协调](../crates/terminal-security/src/vault.rs)、[凭据存储](../crates/terminal-storage/src/credentials.rs)。

分组迁移防环并校验 revision。默认删除将直属主机/子组上移；连同内容删除在单事务中递归删除子树并登记专属凭据清理。公共凭据、信任及已有会话快照保留；删除主机使历史/转发外键置空，工作区 JSON 引用保留为失效。

主机手动顺序作为本机浏览元数据持久化；配置编辑保留已有位置，新建及云端新增合并项默认追加。批量排序/迁组提交完整清单 ID 与 revision，Rust 在单一写事务校验完整范围、重复 ID、版本及目标分组，任一冲突或写入失败整体回滚；只更新顺序/分组和受影响记录版本，不改认证、密码或现有会话。成功返回同事务中的主机/分组快照。手动顺序不进入现有白名单云包，不改变云包格式。

最近使用时间与手动顺序独立持久化，切换排序偏好只改变展示。保存主机的新终端会话（含重连及工作区叶子）、独立 SFTP 和转发通过配置、配额等同步校验并被接受时记录使用，即使后续连接失败也保留该次使用；工作区本身在成功恢复后记录。重复请求、临时连接和本地终端不更新保存项。时间更新不改变配置 revision、配置修改时间或手动位置，前端收到轻量通知后热应用最近使用顺序。旧项及云端新增合并项保留未知时间，不从配置修改时间推断；最近使用时间只属本机元数据，不进入白名单云包。

Rust/TS 主机契约不包含 `favorite`，查询与展示不使用收藏排序。为保持已应用迁移及旧云包兼容，历史数据库和备份中的同名列保留为不参与业务的兼容字段；新建写入 0，编辑不改旧值，既有手动顺序保留。迁移 8 的首次顺序初始化保留原始字节。

主机清单在同一读事务取得主机、标签、认证摘要与分组，前端不得拼接两个独立时点的主机/分组读取。成功写入使更早的前端读取失效，防止迟到快照覆盖已提交结果；请求代次只隔离 UI 旧读，数据库写入仍依赖 revision 和事务校验。

SSH `config_json.startup` 使用共享 `SshStartup` 类型：Shell 或 PTY exec。缺失按 Shell，null/未知类型/无效命令拒绝，非 SSH 不接受 exec。该字段不改变 schema；工作区下次打开读取最新主机配置，不更新活动会话。启动行为见 [SSH](05-ssh-design.md)。

## 4. 物理备份与恢复边界

手工物理备份应在应用退出后保留整个应用数据目录，包括加密数据库、配套 vault.json、可能存在的 WAL/SHM 与改密恢复目录；不能只复制数据库或以普通 `VACUUM INTO` 代替加密备份。密码与 OS 主密钥均丢失，或缺少配套派生元数据时，现有接口无法恢复。

改密只更换当前数据库密钥，既有物理副本仍用原密码；[第三方云备份](22-data-backup.md)使用独立密码及白名单逻辑快照，不包含来源主密钥/vault.json。当前路径不打开或自动清理旧 `termeta.sqlite3`、`termeta-credentials-v1.sqlite3`，旧明文库不会自动加密。

## 5. 设置、Snippets 与会话日志

应用设置保存在 `settings.key=app` 的版本化 JSON，最大 256 KiB。Rust 校验版本、白名单、字段及方案引用；main/settings 使用专用串行队列读取最新值并局部合并，hostsView/hostSearch 逐字段合并，hostSchemes 按主机 ID 局部合并（补丁中的 null 删除指定覆盖，持久化记录不保留 null），旧草稿不能覆盖即时偏好。主机排序和 SFTP 左右停靠偏好保存在同一设置记录，旧值缺失时分别默认手动排序、右侧停靠。提交后通知其他窗口重读。持久化兼容与校验见 [settings.rs](../src-tauri/src/settings.rs)，UI 默认与投影见 [settings/state.ts](../src/modules/settings/state.ts)；搜索查询与拼音索引只驻内存。

云认证及备份密码另存于 `settings.key=backup`、`value_version=2`，不通过普通设置返回，也不进入云备份。专用读取仅返回秘密存在标记；版本 1 配置可保留认证并补填密码后升级，不改变业务 schema。详见 [备份配置](22-data-backup.md#5-配置与权限)。

`snippets` 保存命令片段；`session_logs` 只在终端视图或软件关闭时保存 xterm 带配色/样式的现存缓冲快照及无主机外键的基础连接副本。连接创建/断开不写日志，日志无自动数量裁剪，正文不进入云备份。旧连接历史表仅作兼容遗留，不再读写或转换。关闭写入以 FULL 同步事务提交成功作为销毁视图/退出的前提；完整契约与异常退出限制见 [Snippets 与会话日志](23-snippets-and-session-logs.md)。

## 6. schema 与迁移

密钥和页面完整性通过后，专用初始化连接接受 `user_version=0` 或同格式 2–11。Snippets/日志采用迁移后幂等建表，不增加 SQLx 迁移或改变 user_version，不改已有迁移及校验记录。父表重建前在迁移事务外关闭外键，SQLx `Migrator::run_direct` 之后恢复并检查外键，通过才发布业务池；失败保持回滚与校验和检查。

数据库引擎版本、SQLCipher 加密格式和业务 schema 分别维护。连接保持 SQLCipher 4 兼容参数，解锁仅补齐当前程序内嵌的业务迁移；已经是 schema 11 的库不会因驱动或引擎版本变化而增加 schema。现有入口不自动调用 `cipher_migrate` 或更换主密钥，密码变更仍通过独立的加密副本与恢复流程完成。

| 迁移文件 | 结果 schema | 内容 |
| --- | --- | --- |
| `0001_initial.sql` | 2 | 加密格式 2 初始业务表 |
| `0002_telnet.sql` | 3 | 重建 connection_profiles 增加 Telnet，保留关联和 ID |
| `0003_forwarding.sql` | 4 | 增加 port_forwards，主机删除时引用置空 |
| `0004_workspaces.sql` | 5 | 增加 Workspace、处理历史重名并创建名称唯一索引 |
| `0005_profile_name_index.sql` | 6 | 唯一索引改用 trim(name) |
| `0006_official_sync.sql` | 7 | 已应用的历史迁移，保持原始字节 |
| `0007_remove_official_service.sql` | 8 | 清理废弃表及设置，不改变业务数据或备份格式 |
| `0008_host_order.sql` | 9 | 增加主机手动顺序，首次按原收藏/名称规则初始化，不修改已有迁移 |
| `0009_cloud_bindings.sql` | 10 | 独立保存云账号绑定身份及显示摘要，不包含密码、登录 token 或云端正文 |
| `0010_host_last_used.sql` | 11 | 增加独立最近使用时间，旧项保留未知时间，手动顺序不变 |

新库应用全部迁移，同格式旧库补齐缺少项并升级至 schema 11，文件名、主密钥及元数据不变。**已经应用的迁移必须保持原始字节，后续变更新增迁移，不改 `_sqlx_migrations`。** 升级后不能交回不识别该 schema 的旧应用；回退使用升级前一致备份。

dev 可将当前 cloud 的 schema 10 原位升级至 11，并保留云账号绑定、既有业务数据、迁移历史、主密钥及配套元数据，不降版本或删表；这只提供数据库升级兼容，不引入 CLOUD 产品功能。cloud 同步迁移 10 的编号与原始字节并支持 schema 11 后才可重新切换，不能将已升级数据库交给仍只识别 schema 10 的 cloud。云绑定不进入既有第三方备份白名单。

SQLx 继续严格核对已应用迁移，不忽略缺失迁移或校验和不符。兼容范围不覆盖不识别当前 schema 的旧二进制、未知迁移或未来 schema。后续新增迁移须在两分支同步编号与原始字节，不能用同一编号表示不同变更。

## 7. 转发与系统识别状态

`port_forwards` 保存规则及主机引用，不复制秘密或运行状态；启动重读最新 SSH 配置，编辑/删除校验 revision，运行期间拒绝修改。删除主机保留失效规则及已有独立连接，见 [转发](21-port-forwarding.md)。

SSH 图标、系统识别机会及后端令牌位于 `config_json`。后端 claim 在事务中检查开关并登记一次机会，关闭时不消耗，失败不撤销；complete 仅按令牌、目标及默认图标条件更新，不能复活已删除主机。表单不能回写标记/令牌，复制只保留图标。见 [SSH](05-ssh-design.md)。

## 8. 工作区存档与统一名称

四类保存项共享 `profiles_unique_name`：`trim(name)` 后区分大小写、全库唯一。迁移对历史重名保留首项，其余追加无冲突序号，不改 ID、关联或合并记录。

`save_workspace` 新建不带 expectedRevision；同名替换仅允许 workspace 且 revision 匹配，保留 ID/分组/顺序。`save_host` 对工作区只允许受 revision 保护的分组修改。

工作区叶子是保存 SSH/Telnet/Serial 的 ID，不是独立外键行。保存时事务校验引用，拒绝失效或嵌套工作区；删除主机后不按名称重绑。存档不含秘密、sessionId、输出或广播；打开仍经真实连接入口校验。见 [工作区](18-terminal-workspaces.md)。

## 9. 已应用迁移校验和不一致的恢复

`0004_workspaces.sql` 保持首次实际应用的原始字节，SHA-384 为 `cee4f9328cb293cb70c2c2f2009d0dadcb0a80b73e843a2fad34e421f3801517fb71f65f81061409bf2871ed3ded0f29`。索引调整由迁移 5 完成，schema 5 可升级至 6，SQLx 继续校验全部已应用迁移。

若遇 “migration 4 was previously applied but has been modified”，完全退出应用并备份整个应用数据目录，再由用户重新编译/启动包含当前迁移的版本；迁移在编译期嵌入，旧二进制不会读取恢复后的 SQL。macOS 默认目录为 `~/Library/Application Support/io.termeta/`。不要删除数据库、vault 元数据、钥匙串、迁移记录或手工改校验和；仍不匹配时保留备份并核对实际应用版本。

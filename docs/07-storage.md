# 数据存储与迁移设计

日期：2026-10-15。SQLite + SQLx、OS 凭据适配与首期数据用例已落代码，状态为“实现待验证”。本轮只做源码与格式等静态检查，未启动应用、创建实际业务数据库或执行迁移；后文区分当前实现与后续设计。

## 1. 数据边界

应用数据放在 Tauri 按平台解析的应用数据目录下，使用现有 identifier `com.h3110w0r1d.termvexa`；不硬编码用户目录，不默认放在仓库内。当前数据库为 `termvexa.sqlite3`，迁移前备份暂与数据库同目录，以 `termvexa.backup-<Unix毫秒>.sqlite3` 命名，保留最近 5 份；备份/诊断分目录仍属后续整理。凭据由 OS 安全存储管理。

| 存储 | 内容 | 禁止 |
| --- | --- | --- |
| SQLite | 主机/分组、设置、凭据元数据引用、主机公钥信任、连接历史 | 密码、私钥明文、私钥口令、OTP、持续终端输出 |
| OS 凭据库 | 用户主动保存的密码、私钥解密口令 | 当不可用时偷偷回退 SQLite |
| 用户私钥文件 | 保持在用户选择的原路径，由 Rust 按授权引用读取 | 前端读取或自动复制到数据库 |
| 内存 | 短期认证答案、解密私钥、有限输出队列、跨重连保留的有限终端滚屏/全屏快照 | 无限缓存、将秘密派生到全局状态 |
| 用户导出 | 后续显式导出配置/日志 | 默认包含凭据、可信主机自动导入 |

SQLite 默认未加密，主机名、地址、用户名、路径、历史也可能敏感；OS 凭据库不能保护这些元数据。首期依赖系统用户/磁盘保护，Vault 是后续独立设计。

## 2. 表结构概览

IDs 使用 UUID 字符串；时间用 UTC Unix 毫秒 INTEGER；UI 按本地时区显示；长期累计字节计数使用数据库 64 位整数，IPC 超出 JS 安全范围时使用十进制字符串。

| 表 | 关键字段 | 约束/说明 |
| --- | --- | --- |
| `host_groups` | id、parent_id、name、sort_order、created_at、updated_at、revision | parent 自引用，防环由应用事务验证 |
| `connection_profiles` | id、kind、name、group_id、favorite、config_version、config_json、revision、时间 | 后续 local/serial 复用；JSON 只保存非秘密协议配置 |
| `ssh_profiles` | profile_id、address、canonical_host、port、username、algorithm_policy_id? | 一对一，kind=ssh；高频搜索字段结构化 |
| `profile_auth_steps` | id、profile_id、sort_order、method、credential_id?、key_file_id?、passphrase_credential_id? | ordered plan；password/privateKey/keyboardInteractive |
| `key_files` | id、path、display_name、content_fingerprint、时间 | 引用用户授权文件；无私钥内容；权限与变更重验 |
| `tags` / `profile_tags` | tag id/name；profile_id/tag_id | 唯一标签名、复合主键关联 |
| `credentials` | id、kind、label、store_backend、store_account、status、时间 | 仅 OS item 定位；status=pending/ready/deleting/unavailable |
| `known_hosts` | id、canonical_host、port、key_algorithm、public_key、fingerprint_sha256、trusted_at、last_seen_at | 完整公钥 BLOB 为身份依据；同 endpoint 可显式保存多 key |
| `settings` | key、value_json、value_version、updated_at | 每项有 schema；keymap 用户覆盖、配色默认引用、SFTP 停靠偏好；禁止秘密 |
| `terminal_color_schemes` | 目标结构，当前未建表：id、name、schema_version、colors_json、source_info?、revision、时间 | 仅用户/导入方案；内置方案随只读资源；颜色白名单校验 |
| `profile_terminal_preferences` | 目标结构，当前未建表：profile_id、scheme_ref?、font_override?、revision | 非协议终端偏好，不混进 SshConfig；scheme 引用由事务验证 |
| `session_history` | id、profile_id?、kind、display_name_snapshot、endpoint_snapshot?、started_at、ended_at?、end_reason?、error_code? | 只记连接元数据；默认保留 90 天，可清空 |
| `snippets` | 后续：id、name、body、group_id?、时间 | M3 创建；代码片段也可能含秘密，需显式提示 |
| `workspaces` | 后续：id、name、layout_version、layout_json、时间 | M3 创建；terminal/sftp Tab 判别类型、双栏端点/路径引用、停靠；无 live handle |
| `transfer_history` | M3：id、端点摘要、可选路径、起止时间、字节、结果、错误码 | 可关闭/清空，默认不保存完整路径；无文件正文、凭据或可重放的覆盖同意 |

为何不只有 `hosts`：产品将来有 Local/Serial，“SSH 主机字段必填”会污染其他类型。通用 profile 与 SSH 扩展表分离；UI 仍称“主机”。`config_json` 只放低频协议选项，address/port/username 以 ssh_profiles 为唯一权威来源，禁止在 JSON 再存一份造成双写分歧。

`profile_auth_steps` 的认证方法与引用列承担 CHECK/FK 校验；当前同时保存 typed `AuthStep` 的 `step_json` 供序列化恢复，写入只能经过 Rust 枚举，不允许密码、口令或交互答案进入 JSON。序列化 API 时再组合成版本 1 的 `SshConfig`。引用约束：密码步骤只有 credential_id；私钥步骤必须 key_file_id 且可选 passphrase ref；交互步骤不持久化答案。数据库 CHECK + 应用 typed validation 共同保障，不只依赖 UI。

## 3. 首批 DDL 约束示意

以下保留关键约束示意；初始 DDL 见 [`0001_initial.sql`](../crates/terminal-storage/migrations/0001_initial.sql)，私钥内容授权摘要通过 [`0002_key_file_fingerprint.sql`](../crates/terminal-storage/migrations/0002_key_file_fingerprint.sql) 追加。后续已发布迁移只能追加，不重写校验和。

```sql
CREATE TABLE connection_profiles (
  id TEXT PRIMARY KEY NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('ssh', 'local', 'serial')),
  name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 128),
  group_id TEXT REFERENCES host_groups(id) ON DELETE SET NULL,
  favorite INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1)),
  config_version INTEGER NOT NULL DEFAULT 1,
  config_json TEXT NOT NULL DEFAULT '{}',
  revision INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE ssh_profiles (
  profile_id TEXT PRIMARY KEY NOT NULL
    REFERENCES connection_profiles(id) ON DELETE CASCADE,
  address TEXT NOT NULL,
  canonical_host TEXT NOT NULL,
  port INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
  username TEXT NOT NULL
);

CREATE TABLE known_hosts (
  id TEXT PRIMARY KEY NOT NULL,
  canonical_host TEXT NOT NULL,
  port INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
  key_algorithm TEXT NOT NULL,
  public_key BLOB NOT NULL,
  fingerprint_sha256 TEXT NOT NULL,
  trusted_at INTEGER NOT NULL,
  last_seen_at INTEGER,
  UNIQUE (canonical_host, port, key_algorithm, public_key)
);
CREATE INDEX known_hosts_endpoint ON known_hosts(canonical_host, port);
CREATE INDEX profiles_group ON connection_profiles(group_id, name);
CREATE INDEX ssh_search_address ON ssh_profiles(canonical_host, port);
```

SQLite 外键无法表达的 kind/扩展表一致性、分组防环、JSON schema 版本校验由事务应用服务保障；必要时加触发器，但不把所有业务逻辑藏在 trigger 中。已存 future kind 可识别但未注册工厂时禁止连接，不能错误当成 SSH。

## 4. Repository 与并发

- 当前 core 定义 `HostTrustStore` / `CredentialStore`；`SqliteStore` 直接实现前者，`CredentialVault` 实现后者。主机、分组、设置与历史目前通过 `SqliteStore` 的模块化方法提供，并未另建 `ProfileRepository` / `SettingsRepository` / `HistoryRepository` trait。UI 只调用壳层用例。
- 参数化查询和字段白名单；前端不能传任意 SQL、文件数据库路径或排序表达式。
- 小连接池初始 2 个连接，WAL、每连接 foreign_keys=ON、busy_timeout 初始 3 秒；当前 `synchronous=FULL`；断电一致性、锁竞争和性能尚未实测。
- 写入事务短小，按用例提交；主机保存、分组防环/上移、信任重查与凭据解绑检查使用 SQLx 0.8.6 `Pool::begin_with("BEGIN IMMEDIATE")`，由 SQLite 写事务串行化，读后写检查在同一事务内。单条 revision 条件更新/删除保持 SQL 原子性；Repository 不持应用全局 Mutex 等待数据库，迁移、备份与凭据库等待也不占全局 session 锁。
- 主机与分组修改使用 revision 乐观锁，冲突返回 `REVISION_CONFLICT`；当前不在错误体内附带最新实体，调用方需重新加载，禁止悄悄覆盖其他视图的修改。
- 保存/删除主机不改变已连接 session 的配置快照；新连接读最新 profile；删除时 history.profile_id 置空保留必要摘要。
- 分组移动要事务内验证祖先链；删除默认把子项移到父级后删除；共享凭据采用 RESTRICT/显式解绑策略，不级联删除 OS item。

## 5. 凭据库与数据库的一致性

两者无法单个事务提交，必须考虑崩溃与部分成功。

保存流程：分配 credential UUID → SQLite 写 pending 元数据 → 通过串行凭据工作器写 OS item（service 为应用标识，account 为 UUID）→ SQLite 更新 ready → 保存主机时再校验并绑定认证步骤。当前最后两步是独立用例：若主机保存失败，已保存但未绑定的凭据保留在管理列表，用户可删除。任一步失败保留可解释状态并补偿；pending 不可供连接使用。若用户仅本次使用，不创建持久化记录。

启动只针对本数据库已知 pending/deleting item 进行定向协调，不扫描或导出整个系统密码库。pending item 可以通过用户授权重试/删除；不凭“没有 DB 记录”批量删其他凭据。

删除流程：检查引用与用户选择 → 标 deleting/解绑计划 → 删除 OS item → 删除元数据。OS 不存在可视作已删除；锁定/拒绝则保留待处理状态，不能 UI 说“已删除”而实际留存。当前 `CredentialVault` 将所有 OS 凭据操作串行化，范围比逐 key 串行更保守，且等待 OS 不占用 session 全局锁。

私钥初期仅引用文件；保存的口令与文件 metadata 引用分离。注册时经同一套限大小/no-follow 读取保存实际文件字节的 SHA-256 摘要；使用引用时再读并比对，内容变更即拒绝，须通过原生选择器重新授权。摘要仅用于私钥文件授权变更检测，不是 SSH 主机公钥指纹，不保存私钥正文；摘要字段对 IPC 序列化隐藏。文件失踪、符号链接或权限变化也拒绝，不把路径授权当成永久可信内容。文件读入/解析设置大小上限（初始 1 MiB）和格式限制。

## 6. 迁移与恢复

1. 首次打开确定 app data 路径与权限；进程内串行初始化，后续明确单实例/多实例策略。M2 默认单实例数据库写入入口，不能靠无约束双进程同时升级 schema。
2. 检查数据库 schema 与应用支持范围；遇到未来版本拒绝写入，提示使用新版本，不做猜测降级。
3. 当前首批升级前使用参数化 `VACUUM INTO` 创建包含 WAL 可见内容的一致快照，不直接复制活跃 DB。当前仅有 schema 1；追加新 migration 时必须同步支持版本与备份触发条件，不能直接沿用仅 `user_version=0` 时备份的首批条件。
4. 使用 SQLx 迁移记录与校验和，已发布 migration 不重写，只追加；在支持的事务边界应用。
5. 失败时保留原 DB/备份和脱敏诊断，不自动清库。恢复由用户选择，不能循环覆盖最后有效备份。

备份初始保留最近 5 次，路径可查；备份不包含 OS 凭据秘密，恢复后可能需重新提供凭据引用。导入配置不能自动信任 known_hosts，也不能自动连接或执行 snippet。

## 7. 数据维护

当前启动时按 90 天清理历史，列表最多返回最近 1000 条；可配置保留期/持久化条数上限尚未实现。异常退出留下未 ended 的历史，下一次启动标 `appInterrupted`，不可恢复为 connected。设置和窗口尺寸可以节流写入；终端输出不进入 DB。提供清空历史、删除信任与凭据管理，分别显示影响范围。删除 SQLite 行并不承诺从存储介质安全擦除，尤其 WAL/备份也可能保留元数据。

这里的 session_history 是持久化连接元数据，不是 xterm 的显示历史。断线/失败/重连时显示历史仍由同一 terminalId 的内存缓冲保留，不随后端记录或 tombstone 淘汰；本轮不新增终端正文落盘表。全屏快照采用有界内存策略并纳入视图预算，用户关闭 Tab/窗口才释放。

## 8. 快捷键、配色与 SFTP 偏好

目标契约是按 `value_version` 保存已注册动作的用户覆盖/显式禁用，并在升级时合并新默认。当前先把完整 `keymap.bindings` 与其他首期设置保存为单条 `settings.key=app`、`value_version=1` JSON；仅保存覆盖、默认合并迁移仍需后续完善。导入先做冲突预览，保存原子更新；系统后台注册状态是运行态，不能因 DB 有配置就显示注册成功。模型见 [14](14-shortcuts.md)。

当前内置方案使用固定 ID，自定义/导入方案使用 UUID；`customSchemes`、`defaultSchemeId`、`hostSchemes`（profile ID → scheme ID）均保存在上述 `app` JSON，不创建独立配色/主机终端偏好表，不提供独立 revision/FK。壳层 `settings::validate` 校验配色引用与字段；设置整体原子替换，但尚无设置乐观锁，多窗口扩展前需处理覆盖冲突。主机删除后的偏好引用清理与数据库级 profile 引用完整性仍需补齐。当前 terminalId 的临时覆盖只驻内存；工作区需要恢复时持久化显式 scheme 引用，不保存实际输出。配色 JSON 见 [15](15-terminal-themes.md)，用户方案升级不覆盖。

以下是 M3 扩展设计，当前没有 SFTP 表、传输历史或文件服务。SFTP 偏好计划记录默认左/右/底部位置、显隐/尺寸、隐藏文件/排序，允许 profile 或视图覆盖。独立双栏布局只存端点类型/profileRef 与用户选择保存的路径；本地 rootGrant/connectionRef/entryRef 是运行授权，重启不能直接复用。重新选择/授权后才打开，默认不连接远端。

传输任务为运行态；历史只留可选最小摘要，路径也属于敏感元数据。重启把残留任务记为 interrupted，不自动执行上传/覆盖/删除、不自动信任旧冲突 challenge。原生拖出 staging 仅保存应用管理的残留登记用于安全清理，文件正文不进 DB/备份；普通远端中转不落本地磁盘。详见 [16](16-sftp-workspace.md)。

## 9. 本轮实现路径与验证记录

| 路径 | 当前实现 |
| --- | --- |
| [`terminal-storage/src/lib.rs`](../crates/terminal-storage/src/lib.rs) | 只读预检 `user_version` / SQLx 迁移最高版本、2 连接池、WAL/FULL、SQLx 迁移与校验和、迁移快照/保留 5 份、Unix DB/备份 0600 |
| [`hosts.rs`](../crates/terminal-storage/src/hosts.rs)、[`groups.rs`](../crates/terminal-storage/src/groups.rs) | 主机/分组 CRUD、复制、收藏、标签、名称/地址/用户/标签搜索、revision、分组防环/删除上移；搜索当前在 Rust 侧过滤 |
| [`metadata.rs`](../crates/terminal-storage/src/metadata.rs) | known_hosts 完整公钥比较/撤销、事务内拒绝并发信任不同密钥、设置、私钥引用、连接历史；持久信任管理暂不提供同 endpoint 新增不同 key 的单独用例 |
| [`credentials.rs`](../crates/terminal-storage/src/credentials.rs) | 凭据元数据状态、就绪与类型校验、共享引用拒删 |
| [`terminal-security/src/vault.rs`](../crates/terminal-security/src/vault.rs) | `keyring=3.6.3`，OS 操作 `spawn_blocking` 串行执行，pending 恢复与 deleting 定向协调，无明文 fallback |
| [`key_files.rs`](../crates/terminal-storage/src/key_files.rs)、[`private_key.rs`](../crates/terminal-security/src/private_key.rs) | 共用原生选择/使用读取边界，限 1 MiB、OpenSSH 文本、内容 SHA-256 授权摘要；Unix `O_NOFOLLOW`、uid/权限/设备与 inode 校验 |
| [`src-tauri/src/commands/data.rs`](../src-tauri/src/commands/data.rs)、[`native.rs`](../src-tauri/src/commands/native.rs) | 受控数据/信任/凭据/历史/设置用例，原生私钥选择注册；前端不可传任意数据库或私钥路径 |

固定直接依赖 `sqlx=0.8.6`、`keyring=3.6.3`，已下载对应 release 源码核对所用 features/API 与 MIT OR Apache-2.0 许可证；keyring 声明 MSRV 1.75，SQLx 顶层 manifest 无明确 rust-version，传递依赖与本仓库整体 MSRV 仍须验证。Linux 使用 Secret Service（同步后端、Rust crypto、vendored DBus），不是 mock 或 SQLite 密码替代。

静态检查：两个 crate 的 Rust 格式检查、Cargo TOML 解析、尾随空白与末尾换行检查通过；检查了壳层调用与 DTO 对齐。受仓库约束，未执行编译、运行、数据库迁移演练、崩溃补偿测试、系统凭据库访问或 SSH 连接。SQLx 锁文件/传递依赖安全公告复核、Windows/macOS/Linux 的权限、备份/恢复、锁定/拒绝凭据库行为仍为待验证项。当前没有会话正文日志、会话回放或终端正文落盘；不能将连接历史列表当作这些功能。

本轮补充：私钥授权内容摘要使用 `sha2=0.10.9`，初始迁移尚未发布，直接补充 `key_files.content_fingerprint`；并未对任何已运行数据库改写迁移。哈希处理借用 `Zeroizing<Vec<u8>>`，私钥文本只以 `Zeroizing<String>` 返回原生认证使用，内容不进入 SQLite 或 IPC。该变更只经静态检查，文件替换/重新授权行为仍待授权后的运行验证。

并发约束修正：已删除 `SqliteStore.writes` 应用全局互斥锁，复合写用例改为 `BEGIN IMMEDIATE`；核对 SQLx 0.8.6 `Pool::begin_with` release 源码并执行 Rust 格式静态检查，未做并发/锁竞争运行测试。OS 凭据 `operations` 为专用凭据工作器的业务串行化，不承载全局 session 或 Repository 锁。

## 启动迁移修复（2026-10-15）

用户 macOS 日志报告 setup 中迁移失败。只读检查本机数据库的 `_sqlx_migrations` 与 `key_files` 列信息，确认已应用的 v1 不含 content_fingerprint；此前修改了同版本 SQL 导致校验和不匹配。已按原校验和恢复 0001，新增 0002 ALTER TABLE，支持原 v1 升级与新建数据库顺序执行。SCHEMA_VERSION=2；已有数据库仅在存在待应用迁移时创建一致性备份，未来版本仍只读拒绝。没有修改或删除用户数据库，也没有改写迁移记录以跳过校验。

旧私钥引用迁移后摘要为空，使用时明确拒绝，必须重新选择授权；不会在迁移时自动读取用户私钥。迁移错误保留 SQLx 的具体错误说明，避免仅出现“数据库迁移失败”。新增升级保留数据/备份/旧引用拒绝和新建数据库两项 Rust 回归测试源码，因禁止主动编译未执行。

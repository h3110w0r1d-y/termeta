# 数据存储与凭据设计

日期：2026-10-15。按用户确认的新模型重构，状态为“实现待验证”。本轮不做旧数据库兼容升级；重写初始 schema，不自动删除、转换或重置用户现有数据库。编译、应用启动、实际数据库与系统凭据库均未验证。

## 1. 数据边界

应用数据使用 Tauri 平台应用数据目录，新凭据模型数据库为 `termvexa-credentials-v1.sqlite3`（文件名标识模型代际，内部 schemaVersion 当前为 1）；OS 凭据 service 为 `com.h3110w0r1d.termvexa`。SQLite 不加密，主机地址、用户名、授权文件路径和历史仍属敏感元数据。

| 存储 | 内容 | 禁止 |
| --- | --- | --- |
| SQLite | 主机、分组、公共/专属凭据元数据与引用、设置、信任和连接历史 | 密码、私钥正文、解密密码、OTP、终端正文 |
| OS 凭据库 | 可复用密码；私钥正文与可选解密密码的同一秘密记录；主机专属密码 | 凭据服务不可用时回退为明文 |
| 用户私钥文件 | 仅作为用户授权的导入来源，Rust 受控读取 | 前端读取正文、连接时依赖原文件仍存在 |
| 内存 | 表单短期输入、解密材料、认证答案、有限终端缓冲 | 秘密写入 Pinia 持久状态、日志或导出 |

私钥凭据保存成功后由 OS 凭据库提供完整材料；删除或移动导入来源文件不应破坏已导入凭据。SQLite 的 key_files 只保存授权来源和内容摘要，不保存私钥。

## 2. 两类可复用凭据

凭据库仅提供 `password` 与 `privateKey` 两种类型，不存在独立的 passphrase 凭据。

- 密码凭据：名称与密码，供多个主机显式引用。
- 私钥凭据：名称、从原生选择器授权导入的 OpenSSH 私钥，以及可选解密密码。私钥和解密密码共同归属于一个凭据 ID，秘密不回传 UI。未提供或提供错误解密密码时，连接仍可通过本次 challenge 补充，补充答案不自动写回。

公共列表只返回 id/kind/label/status，不包含主机专属项、私钥正文或任何密码。引用只能匹配正确 kind 且处于 ready 状态。已被主机引用的公共凭据拒绝删除，需先解绑。

## 3. 主机专属密码

主机密码认证有两条路径：引用公共密码凭据，或直接填写密码。直接填写不创建可复用条目：非空密码在保存主机时写入 OS 凭据库，并与该主机 ID 绑定；显式提交空字符串会移除该主机旧密码，连接时询问。没有“记住密码”或“保存口令”复选框。

编辑时不读取原密码回填，也不用假星号代替真实值；只返回 `hasPassword`，界面说明留空保存会改为连接时询问。主机公钥认证引用公共 privateKey 凭据，无单独的口令引用。

收藏等元数据更新不提交 password 字段，表示保留已有主机专属密码。编辑表单总显式提交字符串，空值表示清除；不得将这两种语义合并成默认空字符串。

主机复制保留公共引用，但不复制专属密码。删除或替换主机专属密码时登记 deleting，再协调删除 OS item；不删除仍被其他主机使用的公共凭据。新连接读取当前配置，现有 session 不被配置修改强制关闭。

## 4. 表结构与契约

初始 DDL 见 [0001_initial.sql](../crates/terminal-storage/migrations/0001_initial.sql)，当前全新 schema 基线为 1；此前 0002 升级脚本不属于新模型。不为旧库改写迁移记录或自动清库。

| 表 | 关键字段与约束 |
| --- | --- |
| connection_profiles / ssh_profiles | 主机元数据、地址/端口/用户、revision；不保存秘密 |
| profile_auth_steps | profile_id、method、credential_id；只允许 password/privateKey，一个主要步骤 |
| credentials | id、kind=password/privateKey、label、owner_profile_id、store_account、status、时间 |
| key_files | id、path、display_name、content_fingerprint；原生选择器签发的导入授权 |
| host_groups / tags / profile_tags | 分组防环、标签与关联 |
| known_hosts | endpoint、算法、完整公钥、SHA256 指纹；不能只按指纹决定信任 |
| settings / session_history | 非秘密偏好、连接摘要；正文不落库 |

`owner_profile_id=NULL` 表示公共凭据；非空只允许 password，表示该主机的专属秘密。该列不随主机删除级联消失，以便保留待清理 OS item 的登记。引用绑定与所有权由同一 SQLite 写事务校验；跨主机专属引用、公共列表泄露专属条目和将专属 ID 当作公共引用均拒绝。

HostProfile 的 `authPlan` 对主机专属密码隐藏内部 credential ID，并提供 `hasPassword`。仅 Rust 的 `get_host_connection` 组合实际会话配置；临时连接不能任意引用主机专属密码。

## 5. 跨存储一致性

SQLite 与系统凭据库不能原子提交。凭据状态为 pending → ready → deleting，未就绪项禁止用于认证。

公共保存：登记 pending → 专用阻塞工作器写 OS item → 标 ready。私钥导入前复核授权文件的限大小、OpenSSH 格式、权限/no-follow 与内容摘要；导入正文和可选解密密码在一个 OS item 保存。

主机密码替换：先创建新的主机专属 pending item，写 OS 并标 ready，再以 revision 与所有权检查事务绑定主机；成功后标旧项 deleting 并清理。保存冲突或 OS 写失败不得破坏原主机引用或旧密码，新未绑定项须补偿或登记待清理。

主机清除/删除：事务解绑并登记 deleting → OS 删除 → 移除元数据。已删除 OS item 视为完成；锁定/拒绝时保留任务，不能先丢失元数据导致无法清理。启动仅定向处理本数据库登记的孤立专属项/deleting，不扫描系统密码库。公共 pending 可由用户恢复或删除；公共恢复/删除接口不能操作主机专属项。

凭据专用工作器可串行化 OS 操作；不能在 session 全局锁或 SQLite 写事务内等待系统凭据库。所有秘密尽早进入 Zeroizing，IPC/第三方库副本与 JS 字符串不承诺完全擦除。

## 6. 并发与新数据库基线

采用 SQLx SQLite 小连接池、WAL/FULL、foreign_keys、busy timeout。复合写使用 BEGIN IMMEDIATE 和 revision 乐观锁，不在应用全局 Mutex 内等待数据库。主机绑定、新旧专属密码切换、引用拒删须在同一事务内复查，避免元数据更新与密码替换竞态。

本轮用户明确排除旧数据库兼容迁移：schema 与 Rust/TS 契约同时重构，移除旧 passphrase/文件认证步骤。桌面启动固定使用独立新文件 `termvexa-credentials-v1.sqlite3`，不打开旧 `termvexa.sqlite3`，避免旧版本 2 与新 schema 1 冲突。新库首次启动为空，主机、设置、信任等需重新配置；旧库与其 WAL/SHM、OS 凭据和备份原样保留，不自动导入或清理。不能通过忽略版本/校验和保护来复用旧库。保留未来版本只读预检、迁移校验与失败保留机制，后续正式发布迁移仍需独立设计。

## 7. 数据维护与后续范围

连接历史初始按 90 天清理、最多列出 1000 条；异常退出的未结束记录标 appInterrupted，不自动重连。终端滚屏按 terminalId 留在窗口内存，与 session_history 分开。SQLite 行删除不表示 WAL、备份和介质中安全擦除。

SFTP/Local/Serial/隧道仍保持原里程碑，不因本轮凭据重构引入实际文件传输或本地终端。

## 8. 快捷键、配色与 SFTP 偏好

目标契约是按 `value_version` 保存已注册动作的用户覆盖/显式禁用，并在升级时合并新默认。当前先把完整 `keymap.bindings` 与其他首期设置保存为单条 `settings.key=app`、`value_version=1` JSON；仅保存覆盖、默认合并迁移仍需后续完善。导入先做冲突预览，保存原子更新；系统后台注册状态是运行态，不能因 DB 有配置就显示注册成功。模型见 [14](14-shortcuts.md)。

当前内置方案使用固定 ID，自定义/导入方案使用 UUID；`customSchemes`、`defaultSchemeId`、`hostSchemes`（profile ID → scheme ID）均保存在上述 `app` JSON，不创建独立配色/主机终端偏好表，不提供独立 revision/FK。壳层 `settings::validate` 校验配色引用与字段；设置整体原子替换，但尚无设置乐观锁，多窗口扩展前需处理覆盖冲突。主机删除后的偏好引用清理与数据库级 profile 引用完整性仍需补齐。当前 terminalId 的临时覆盖只驻内存；工作区需要恢复时持久化显式 scheme 引用，不保存实际输出。配色 JSON 见 [15](15-terminal-themes.md)，用户方案升级不覆盖。

应用标签栏位置保存在 `app.tabPosition`（`top` / `left`），旧设置缺失时前端补为 `top`，Rust 兼容缺失但拒绝其他值。固定首页/SFTP 与当前首页菜单不新增数据库表；该设置不新增数据库表。

以下是 M3 扩展设计，当前没有 SFTP 表、传输历史或文件服务。SFTP 偏好计划记录默认左/右/底部位置、显隐/尺寸、隐藏文件/排序，允许 profile 或视图覆盖。独立双栏布局只存端点类型/profileRef 与用户选择保存的路径；本地 rootGrant/connectionRef/entryRef 是运行授权，重启不能直接复用。重新选择/授权后才打开，默认不连接远端。

传输任务为运行态；历史只留可选最小摘要，路径也属于敏感元数据。重启把残留任务记为 interrupted，不自动执行上传/覆盖/删除、不自动信任旧冲突 challenge。原生拖出 staging 仅保存应用管理的残留登记用于安全清理，文件正文不进 DB/备份；普通远端中转不落本地磁盘。详见 [16](16-sftp-workspace.md)。

## 9. 实现与验证边界

实现涉及 storage 的 schema/model/hosts/credentials、security vault、core CredentialStore 与认证步骤、SSH auth、Tauri 数据/会话命令及前端主机/凭据页。IPC 不增加秘密读取接口，原生私钥选择只返回引用和名称。

静态证据与未执行项见 [实施状态](17-implementation-status.md)。Rust 测试源码用于后续验证新库、引用类型/归属、专属密码替换/清除、复制与拒删；未编译执行。OS 凭据容量限制、锁定/拒绝、崩溃协调、加密/无密码私钥与三平台 SSH 互通仍须运行验收。

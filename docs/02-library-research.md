# 技术选型与依赖依据

更新：2026-10-17。以下为当前源码与锁定发布包的静态核查结果，未代表编译兼容、真实连接或三平台运行验收。具体依赖以 Cargo.lock、pnpm-lock.yaml 为准。

## 1. 当前选型

| 模块   | 当前实现                                                           | 依据与边界                                                                                                                                                                                                                                     |
| ------ | ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 桌面   | Tauri 2.11.5 / build 2.6.3，JS API 2.11.0                          | [Commands](https://v2.tauri.app/develop/calling-rust/)、[Channel](https://v2.tauri.app/reference/javascript/api/namespacecore/)、[Capabilities](https://v2.tauri.app/security/capabilities/)；应用命令须 AppManifest/ACL 与业务 owner 双重校验 |
| 前端   | Vue 3、TypeScript、Pinia 4.1.0                                     | [Pinia](https://pinia.vuejs.org/introduction.html)；仅管理轻量状态，不持有终端或文件正文                                                                                                                                                       |
| UI     | Naive UI 2.45.3                                                    | MIT，Node >=20、Vue ^3.0.0；采用库组件与主题，工作区标签自定义，详见 [UI 选型](13-ui-stack.md)                                                                                                                                                 |
| 终端   | xterm 6.0.0、Fit 0.11.0、Search 0.16.0、WebLinks 0.12.0            | MIT；[Terminal API](https://xtermjs.org/docs/api/terminal/classes/terminal/)、[编码](https://xtermjs.org/docs/guides/encoding/)、[流控](https://xtermjs.org/docs/guides/flowcontrol/)、[安全](https://xtermjs.org/docs/guides/security/)       |
| SSH    | russh 0.64.1，关闭默认 features，ring+rsa                          | Apache-2.0，MSRV 1.89；[发布 API](https://docs.rs/russh/0.64.1/russh/)、[维护者仓库](https://github.com/Eugeny/russh)                                                                                                                          |
| 异步   | Tokio / tokio-util，共用 Tauri runtime                             | [spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)；长期阻塞与短期凭据工作分别管理                                                                                                                               |
| 数据库 | SQLx 0.8.6，关闭默认 features，sqlite/runtime-tokio/migrate/macros | MIT OR Apache-2.0；[发布 API](https://docs.rs/sqlx/0.8.6/sqlx/)；Rust 独占 SQL，无前端任意 SQL                                                                                                                                                 |
| 凭据   | keyring 3.6.3                                                      | MIT OR Apache-2.0，MSRV 1.75；[发布 API](https://docs.rs/keyring/3.6.3/keyring/)；macOS apple-native、Windows windows-native、Linux sync-secret-service/crypto-rust/vendored                                                                   |
| SFTP   | russh-sftp 3.0.1                                                   | Apache-2.0；[发布 API](https://docs.rs/russh-sftp/3.0.1/russh_sftp/)；独立 subsystem 流，不混入 PTY                                                                                                                                            |

xterm.write 接收原始 Uint8Array，解析回调用于消费 ACK，不等同 GPU 呈现；不使用 HTTP/WebSocket 或 addon-attach。WebGL 不是可用性前提。认证、PTY/Shell 请求须等服务器接受回复，不能将发送成功等同连接成功。

keyring 当前采用 3.6.3 的平台适配器；[keyring-core](https://docs.rs/keyring-core/latest/keyring_core/) 为后续候选，不混用其 API。系统服务不可用时禁止明文或 mock 回退。秘密尽早进入 Zeroizing；钥匙串仅保存一把主密钥，私钥正文与可选解密密码封装在同一加密数据库凭据项，serde/serde_json/uuid 使用根锁定版本，不回传前端。

## 2. SFTP 与原生拖放

russh-sftp 的 RawSftpSession 提供增量目录与文件流 API，避免高层 read_dir 全量收集。3.0.1 正式依赖不绑定 russh，通过 AsyncRead+AsyncWrite 对接当前 SSH 子系统。包上限 256 KiB，单请求 15 秒超时，读写块 64 KiB，全局最多 4 个任务。上传通过打开后属性核对及 fsetstat 截断已确认的普通文件；新文件独占创建，同名目录仅合并，不依赖远端 hardlink。

上游未声明 MSRV，锁解析不证明编译兼容。其依赖下界要求 bytes 1.12.1、serde 1.0.229、tokio-util 0.7.19；根锁文件已同步。最新 RustSec 全量复核受接口访问限制未完成，不宣称依赖安全审计通过。

macOS 原生文件/目录承诺使用 [objc2 0.6.5](https://docs.rs/objc2/0.6.5/objc2/)、[block2 0.6.2](https://docs.rs/block2/0.6.2/block2/)、[objc2-app-kit 0.3.2](https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/struct.NSFilePromiseProvider.html)、objc2-foundation 0.3.2，限 macOS target、按需 features。声明 MSRV 1.71；objc2/foundation/block2 为 MIT，AppKit 为 Zlib OR Apache-2.0 OR MIT。主线程创建拖动会话，NSOperationQueue 后台兑现文件，复用有界 Rust 下载。原生拖入 [Tauri 事件](https://v2.tauri.app/reference/javascript/api/namespacewebview/) 的平台坐标在壳内统一为逻辑坐标，不在前端重复除 DPR。双向拖放生命周期与 Finder 行为待运行验证。

Windows/Linux 拖出尚未实现。候选 [drag 2.1.1](https://docs.rs/drag/2.1.1/drag/) / [tauri-plugin-drag 2.1.1](https://crates.io/crates/tauri-plugin-drag/2.1.1) 为 MIT OR Apache-2.0、edition 2021，未声明 MSRV；当前未引入。Windows OLE CF_HDROP 与 Linux GTK URI 需要已存在本地文件，不能直接承诺 SFTP 内容。后续若采用缓存，须限制授权/配额/TTL，准备后由用户重新发起拖动；GTK URI 编码及 X11/Wayland 须单独验证。

## 3. 字体、主题与输入

12 个本地字体家族的正式来源、许可、版本和 SHA-256 见 [字体资源](../public/fonts/README.md) 与 [manifest](../public/fonts/manifest.json)。使用原始字体、不依赖 CDN，不新增运行时字体下载；各字体许可分别保留，不按单一许可证概括。

配色使用 [xterm ITheme](https://xtermjs.org/docs/api/terminal/interfaces/itheme/) 与共享静态方案库，具体映射见 [配色规格](15-terminal-themes.md)。系统后台快捷键仅为后续 opt-in，[Tauri Global Shortcut](https://v2.tauri.app/plugin/global-shortcut/) 不替代应用命令路由和 IME/焦点策略。

固定 xterm 6.0.0 的公开 onData 包含自动应答，onKey 不覆盖全部 IME。inputOrigin.ts 只读订阅内部 coreService.onUserInput，与 onData/onBinary 配合隔离广播来源；能力缺失拒绝广播，升级必须复核内部接口，不能仅凭焦点认定所有输出为用户输入。

## 4. Local、Serial 与 Telnet

| 适配器         | 固定依赖与依据                                                                                                        | 边界                                                                                                                                                                                                                                                                                                                                                                                    |
| -------------- | --------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Local          | [portable-pty 0.9.0](https://crates.io/crates/portable-pty/0.9.0)，MIT，未声明 MSRV                                   | Unix PTY/Windows ConPTY；阻塞 I/O 独立线程，初始进程组回收；完整跨平台进程树待完善                                                                                                                                                                                                                                                                                                      |
| Serial         | [serialport 4.10.1](https://docs.rs/serialport/4.10.1/serialport/)，MPL-2.0、MSRV1.59，禁用 libudev 默认 feature      | 库仅 None/Software/Hardware 流控；Unix TIOCEXCL/flock、Windows 零共享；macOS 通过 IOKit 角色适配筛选                                                                                                                                                                                                                                                                                    |
| 编码           | [encoding_rs 0.8.42](https://docs.rs/encoding_rs/0.8.42/encoding_rs/)，MSRV1.88，(Apache-2.0 OR MIT) AND BSD-3-Clause | 流式解码、非法输出替换，不可表示输入明确失败                                                                                                                                                                                                                                                                                                                                            |
| macOS 角色枚举 | core-foundation 0.10.1、io-kit-sys 0.4.1，均 MIT/Apache-2.0                                                           | 读取 IODialinDevice 精确匹配候选，RAII 处理对象所有权，实际打开保持用户节点                                                                                                                                                                                                                                                                                                             |
| Telnet         | 内部 terminal-telnet + 已锁 Tokio 1.50.0/async-trait/core                                                             | 不调用系统 telnet 或额外协议库；[RFC 854](https://www.rfc-editor.org/rfc/rfc854)、[855](https://www.rfc-editor.org/rfc/rfc855)、[856](https://www.rfc-editor.org/rfc/rfc856)、[857](https://www.rfc-editor.org/rfc/rfc857)、[858](https://www.rfc-editor.org/rfc/rfc858)、[1091](https://www.rfc-editor.org/rfc/rfc1091)、[1073](https://www.rfc-editor.org/rfc/rfc1073) 的有界交互子集 |

上述版本的 API/features/许可证曾按固定发布源码核对；Serial/core 已有 macOS 局部编译/测试证据，其余平台和完整应用兼容性未验收。OSV 对当时 serialport/encoding_rs 固定版本查询未返回公告，不代表全依赖审计。实现和设备边界见 [Serial](19-serial-terminal.md) 与 [Telnet](20-telnet-terminal.md)。

## 5. 未关闭验证

| 编号  | 必须回答的问题                                                                                             | 失败时的处理                                                     |
| ----- | ---------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| SP-01 | russh 同一 release 能否完成密钥校验等待/取消、密码/加密私钥、交互认证、PTY 和 resize？                     | 调整 release/适配层；不能跳过密钥校验                            |
| SP-02 | 三种 WebView 的原始 Channel、解析 ACK、背景标签是否可用且有界？                                            | 调整帧/批次；二进制不可用时经 ADR 改受界的字节序列化方案         |
| SP-03 | keyring 平台后端能否可靠读写/取消并处理无服务/锁定？                                                       | 收紧支持矩阵或暂禁保存；不降低秘密保护                           |
| SP-04 | SQLx SQLite feature、迁移、路径、备份在三平台是否一致？                                                    | 更换版本/隔离存储实现，不暴露 SQL 给前端                         |
| SP-05 | 关闭是否打断满队列/无 ACK/远端无响应？KeepAlive 是否仍正常？                                               | 调整 actor/任务拓扑，未通过不进入 MVP                            |
| SP-06 | PTY/串口在设备阻塞时如何唤醒退出？                                                                         | 评估专用线程、平台取消 API、tokio-serial                         |
| SP-07 | Naive UI/xterm 组合能否在三种 WebView 中保持终端跨重连不清空，并正确处理焦点、Portal、CSP 和旧流模式边界？ | 调整控件/流适配层；不能用清屏或重建终端规避                      |
| SP-08 | 应用内全局/终端按键与点击手势能否配置、消除冲突/重复执行，并兼容 IME/菜单？                                | 调整 router/焦点适配；系统后台另在 M5 验证，不牺牲远端默认控制键 |
| SP-09 | 所选 xterm release 是否映射全部已编辑 palette 字段、可热应用，导入可往返且安全？                           | 调整映射或版本，不能静默忽略颜色，不以重建终端热更新             |
| SP-10 | SFTP-only、双栏各端点组合、metadata/冲突/取消/中转与共享连接公平性是否满足？                               | 调整 provider/独立连接/能力策略；不跳过身份或覆盖检查            |
| SP-11 | 原生拖入/拖出能否在三平台文件管理器工作，staging 生命周期/冲突是否可靠？                                   | 替换原生适配、明确受限平台；需求未通过不得声称完整交付           |

各 SP 项尚无完整三平台验收；SP-06 有局部串口证据，SP-10/11 有部分 macOS 交互反馈，均不能标整项通过。系统后台快捷键属 M5。执行规则和证据记录见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。

发布前须对实际启用依赖重新检查公告、传递 MSRV、许可证 notices 与三平台支持；静态 API/格式检查不能替代这些门槛。

## 6. 加密保险库固定依赖

| 依赖            | 版本与用途                                   | 源码核对与边界                                                                                                                                                      |
| --------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| libsqlite3-sys  | 0.30.1，`bundled-sqlcipher-vendored-openssl` | 内嵌SQLCipher4.5.7（BSD），AES-256-CBC/HMAC-SHA512；SQLx0.8.6共享同一sys链接，vendored OpenSSL。核对Cargo features和内嵌版本；三平台C/OpenSSL工具链与完整公告待验收 |
| argon2          | 0.6.0，`alloc/zeroize`，禁用默认features     | MIT OR Apache-2.0，MSRV1.85；v19 Argon2id固定64MiB/3轮/1并行度、32字节输出                                                                                          |
| aes-gcm         | 0.11.1，`zeroize`                            | MIT OR Apache-2.0，MSRV1.85；仅保险库密钥校验记录使用AES-256-GCM，不自行替代数据库页加密                                                                            |
| rand_core / hex | 0.6.4（getrandom）/0.4.3                     | OS随机盐/nonce；SQLCipher raw-key编码，编码不代替加密                                                                                                               |

来源：[SQLCipher设计](https://www.zetetic.net/sqlcipher/design/)、[libsqlite3-sys固定源码](https://docs.rs/crate/libsqlite3-sys/0.30.1/source/)、[Argon2固定API](https://docs.rs/argon2/0.6.0/argon2/)、[AES-GCM固定API](https://docs.rs/aes-gcm/0.11.1/aes_gcm/)。实际以锁定registry源码核对使用的接口、features、MSRV与许可证；未编译或执行，不据此宣称完成密码学审计或三平台支持验收。

密码修改不新增依赖。已核对锁定 libsqlite3-sys 0.30.1 内嵌 SQLCipher 4.5.7 的 sqlite3_rekey_v2 与 PRAGMA 处理，支持 raw-key rekey；结合[官方 API](https://www.zetetic.net/sqlcipher/sqlcipher-api/)在 DELETE 模式的独占加密副本上使用，并以新密钥重新打开验证，不能只依据 PRAGMA 返回成功。SQLx 0.8.6 的 Pool::begin 返回独立事务，Pool::close 等待连接归还；tokio RwLockReadGuard::try_map 用于数据库切换门闩。上述均为源码/API 静态核对，尚未运行 SQLCipher 改密或三平台故障验证。

## 7. UI、搜索与资源依据

通用 UI 图标使用 [@vicons/ionicons5 0.13.0](https://registry.npmjs.org/@vicons/ionicons5/0.13.0)，MIT、无运行依赖，manifest 未声明 engines/peerDependencies；类型与源码按固定发布包核对，实际体积未测量。favicon 保留 [Ionicons MIT](https://github.com/ionic-team/ionicons/blob/v5.5.4/LICENSE) 声明，图标范围见 [UI 规范](13-ui-stack.md#图标来源与替换范围)。

主机搜索采用 Naive UI NSelect remote/filterable/header/focusInput；静态拼音表和匹配算法来自用户指定 roco-helper-web，无新增 npm 依赖，来源/摘要见 [资源说明](../resources/README.md#主机搜索拼音表)。下拉和 IME 仍需 WebView 验收。

SSH 系统识别使用锁定 russh 的远端标识、独立 session channel/exec 及受界消息；命令资源由 Rust/前端共享 JSON，解析结果只供图标。用户指定素材与配色原始来源/摘要保留在 resources，固定脚本及 UI 只读约束见 [SSH 设计](05-ssh-design.md#10-一次性主机系统识别)。不把资源核对当成许可发布通过。

## 8. 迁移 API 与验证口径

SQLx 0.8.6 的 SQLite 迁移事务由 Migrator 管理，当前已取得连接后使用 run_direct；父表重建的 foreign_keys 切换在专用连接的迁移事务外完成，恢复/检查后才发布池。不是通过 SQL 脚本内 PRAGMA 或 no-transaction 标记绕过事务。完整升级链及回归边界见 [存储](07-storage.md)。

清单/锁解析、下载源码和格式检查不等于编译兼容、漏洞处置或运行验收。完整公告与许可、实际启用依赖、MSRV 和三平台原生工具链须发布前重新核验，已知未关闭风险见 [公告记录](01-product-requirements.md#10-依赖与公告核查)。

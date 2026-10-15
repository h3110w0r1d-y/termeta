# 技术选型与联网预研

核查日期：2026-10-15。方法：阅读官方文档、维护者仓库和 API 文档；没有安装依赖、编译样例或进行真实连接。结论是**文档层面的适配性判断**。

## 1. 仓库事实

| 项目 | 当前声明/内容 | 判断 |
| --- | --- | --- |
| 前端 | Vue `^3.5.13`、TypeScript `~6.0.3`、Vite `^8.0.16` | 保留，不因原建议切换框架 |
| 桌面 | `@tauri-apps/api`/CLI `^2`；Rust `tauri = 2` | 已创建 Tauri 2 项目，不重新初始化 |
| Rust | edition 2024；serde、serde_json、opener | 尚无 SSH、Tokio、存储等业务依赖 |
| 包管理 | 已有 `pnpm-lock.yaml`；Tauri hooks 使用 pnpm | 前端统一 pnpm；不混用锁文件 |
| 安全配置 | CSP 为 null；默认 capabilities；build.rs 为模板 | 后续需要实施 ACL、CSP，与原生业务同步验收 |
| 业务 | App.vue/greet 模板 | 没有已完成的 SSH/Local/Serial 能力 |

声明范围不等于锁定后的实际版本；本文不擅自升级模板。Node/Rust 最低版本及插件组合在 M1 授权验证时统一确认。

## 2. 选型总表

| 模块 | 决策 | 文档证据 | 主要限制/待验证 |
| --- | --- | --- | --- |
| 桌面框架 | Tauri 2，沿用 | Commands、Channels、自定义命令 ACL | WebView 差异、最低系统版本、权限配置 |
| 前端 | Vue 3 + TypeScript；引入 Pinia 管理元数据 | 现有项目、Pinia 官方介绍 | 不将 xterm/输出放入深层响应式状态 |
| UI | Naive UI 2.45.3 现成组件/主题，替代 Reka UI | 已核对 npm 元数据、包内类型/实现、MIT 许可证与 peer/engines | Node >=20、Vue ^3.0.0；三平台 WebView 未验证；终端生命周期独立 |
| 终端 | `@xterm/xterm` + fit/search；WebGL 可选 | xterm 官方 API、addons、编码与流控 | 必须测试 WKWebView/WebView2/WebKitGTK；有非 WebGL 回退 |
| SSH | russh，文档上符合，进入 M1 集成验证 | client Handler/Handle/Channel/Config | 版本更新快、算法与私钥格式需 fixtures、编译后端需核验 |
| 异步 | Tokio + tokio-util 取消信号 | Tokio 官方阻塞任务说明 | 共用 Tauri 提供的 Tokio 执行环境，避免套建 runtime |
| 数据库 | SQLite + SQLx，Rust 端独占 | SQLx sqlite/runtime/migrate 文档 | SQLite C 依赖、feature/MSRV、迁移与锁竞争 |
| 凭据 | `keyring-core` + 平台存储后端，条件采纳 | 新版 keyring 官方推荐 | 三平台后端、Linux Secret Service 缺失/锁定必须验证 |
| Local | portable-pty，后续阶段 | WezTerm 所属库、PTY/Child API | 阻塞 I/O、ConPTY、进程树退出与 fd 生命周期 |
| Serial | serialport 候选；并列评估 tokio-serial | 原始字节、参数、枚举、异步串口 API | 实体设备、Windows 驱动、Linux libudev、退出唤醒 |
| SFTP | russh-sftp，M3 主候选 | 独立子系统、open flags/metadata/读写 API | 版本组合、服务器属性/rename 能力及传输公平性 |
| 快捷键 | 应用命令路由；OS 后台才用 global-shortcut 候选 | Tauri 官方桌面插件 | 插件不管理终端焦点/鼠标手势，不替代自建 keymap |
| 终端配色 | xterm ITheme + 自建方案库/取色编辑器 | 官方颜色字段/API | release 映射、扩展色、导入格式和热更新实测 |
| 文件拖拽 | Tauri native drop；drag-rs 拖出候选 | 官方 WebView API、维护者说明 | Web drop 不代表原生虚拟文件拖出；OS/Wayland 与 staging 待验证 |

## 3. Tauri：适合，但权限需要显式设计

Commands 适合控制操作；官方建议 Channel 承载流式数据。API 文档说明二进制原始响应在 JS 端表现为 ArrayBuffer，可用于终端输出桥接。[Commands](https://v2.tauri.app/develop/calling-rust/) · [Channel API](https://v2.tauri.app/reference/javascript/api/namespacecore/)

重要修正：应用自己注册到 `invoke_handler` 的命令默认对应用窗口/WebView 可用。应通过 `AppManifest::commands` 使自定义命令进入权限体系，再在指定 capability 授权；此外 Rust 仍需校验窗口与 session 所有权。不能把“插件命令默认受权限控制”的结论直接套在应用命令上。[Capabilities](https://v2.tauri.app/security/capabilities/)

决策：不启动额外 HTTP/WebSocket 服务，不使用 addon-attach。Channel 不等于消费背压，应用自行维护 ACK 和信用窗口。

## 4. xterm.js：渲染符合，数据流需要应用补齐

官方仓库包含 fit、search、web-links、webgl 等扩展，使用 `@xterm/*` 包名。WebGL2 是加速选项，不是可用性前提。[官方仓库与扩展](https://github.com/xtermjs/xterm.js)

`Terminal.write` 支持 Uint8Array 与回调；应传原始 UTF-8 字节，避免 Rust 分块 `from_utf8_lossy` 破坏跨帧字符。回调代表该批数据经过终端解析，可作为应用消费 ACK 的依据，不等于画面已完成 GPU 呈现。[Terminal API](https://xtermjs.org/docs/api/terminal/classes/terminal/) · [编码说明](https://xtermjs.org/docs/guides/encoding/)

大量输出会在消费者滞后时积压，官方有专门流控指南。本项目采用 Rust 队列 + IPC 信用窗口 + 解析回调 ACK，而非逐字符 IPC。[流控](https://xtermjs.org/docs/guides/flowcontrol/) · [安全](https://xtermjs.org/docs/guides/security/)

未验证：具体 xterm/addon release 组合、中文/emoji 宽度、输入法、WebGL context 丢失、WebKitGTK 的实测吞吐。接入时按 peerDependencies 选择稳定组合，不使用文档中的旧版 `xterm` 包名。

## 5. russh：采用候选主引擎

文档显示其提供 Tokio 异步 SSH 客户端，具备认证、channel 与流接口。Handler 的主机密钥检查由调用方决定信任，默认拒绝；这与应用信任仓库设计吻合。[crate](https://docs.rs/russh/latest/russh/) · [Handler](https://docs.rs/russh/latest/russh/client/trait.Handler.html)

Handle 包含密码、公钥和多轮 Keyboard-interactive 接口；Channel 提供 PTY、Shell 和窗口变化能力。需要判断协议回复，不能把请求成功入队等同服务器接受。[Handle](https://docs.rs/russh/latest/russh/client/struct.Handle.html) · [Channel](https://docs.rs/russh/latest/russh/struct.Channel.html)

Config 暴露 KeepAlive、缓冲与窗口参数。后端默认缓冲也要纳入总预算；应用背压不能只约束最外层队列。[Config](https://docs.rs/russh/latest/russh/client/struct.Config.html)

维护者仓库介绍了 crypto backend 和扩展能力，但 main 分支不等于选定 release。落地前只启用一个匹配 release 的加密后端，核查默认 features、RSA-SHA2/密钥格式、跨平台构建需求及安全公告，不复制 main README 的 Cargo 片段即宣称可用。[维护者仓库](https://github.com/Eugeny/russh)

首期协议要求：Ed25519、RSA-SHA2，安全默认算法；不承诺覆盖所有老设备。证书、PPK、Agent、跳板、弱算法为后续独立需求。

对比 ssh2：其 Session 文档明确同一底层会话的阻塞读会影响其他操作，可用非阻塞模式解决，但需要额外适配。本项目选择 russh 以贴合 Tokio；这不表示 ssh2 无法实现产品。[ssh2 Session](https://docs.rs/ssh2/latest/ssh2/struct.Session.html)

## 6. Local 与串口：保留能力，不提前接入

portable-pty 提供跨平台 PTY 抽象和子进程句柄，符合本地交互终端方向。PTY reader/writer 不能直接放入异步执行线程；子进程退出、进程树清理与 Windows ConPTY 仍需应用负责。[portable-pty](https://docs.rs/portable-pty/latest/portable_pty/)

serialport 是阻塞式串口库，支持枚举与平台接口；Linux 默认 libudev 可能影响部署，macOS 同一设备可能有 cu/tty 两类端点，不能直接展示为两个物理设备。官方也提示维护者需求；进入串口阶段时需重新评估维护状态。[serialport API](https://docs.rs/serialport/latest/serialport/) · [平台、依赖与许可](https://github.com/serialport/serialport-rs)

tokio-serial 的 SerialStream 提供 AsyncRead/AsyncWrite，是串口阶段候选；是否取代独立阻塞线程取决于取消、控制线与驱动实测，不在 M0 冻结。[SerialStream](https://docs.rs/tokio-serial/latest/tokio_serial/struct.SerialStream.html)

Tokio 官方说明已经启动的 spawn_blocking 任务不能靠 abort 终止，长期阻塞循环宜使用专用线程。本项目将短时凭据操作与长期 PTY/串口读循环分别设计。[spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)

## 7. 存储、凭据与 SFTP

SQLx 提供 SQLite 与迁移支持。采用 Rust repository，不向前端开放任意 SQL。优先动态参数绑定查询，避免初期构建依赖在线数据库；若后续启用 query!，需管理离线元数据。SQLite 不需要为联网数据库引入 TLS 后端。[SQLx](https://docs.rs/sqlx/latest/sqlx/)

**keyring 原建议需要更新**：当前 keyring 文档推荐需要控制平台后端的应用使用 keyring-core 和具体存储库，而非直接套用旧版 keyring 3.x feature 配置。core 内 mock/sample 存储用于示例/测试，不得用作生产安全凭据库。[keyring](https://docs.rs/keyring/latest/keyring/) · [keyring-core](https://docs.rs/keyring-core/latest/keyring_core/)

决策：定义自己的 CredentialStore，候选组合为 keyring-core + macOS/Windows 原生存储 + Linux Secret Service 后端；具体后端包名、feature、线程模型和版本在 SP-03 验证后冻结。不可用时只允许不保存凭据，禁止自动退回文件/明文存储。

russh-sftp 提供可接原始子系统流的 SFTP 实现，适合独立扩展，但不能仅因名称配套就认为任意版本可组合。[russh-sftp](https://docs.rs/russh-sftp/latest/russh_sftp/)

Pinia 适合 Vue 共享状态；仅管理主机列表、会话摘要与偏好。[Pinia 官方介绍](https://pinia.vuejs.org/introduction.html)

## 8. 版本观察与依赖冻结

下表仅记录本轮页面显示值，**不是推荐已安装版本，不代表经过兼容性验证或安全审计**。

| 页面 | 观察值 | 使用方式 |
| --- | --- | --- |
| russh crate 首页 | 0.64.1 | 子页面搜索索引仍出现 0.63.3，不能混用签名 |
| portable-pty | 0.9.0 | M4 前重新确认 |
| serialport | 4.10.1 | M4 前重新确认 |
| russh-sftp | 3.0.1 | M3 选组合 |
| SQLx | 0.9.0 | 与实际 Rust 工具链、SQLite features 核验 |
| keyring / keyring-core | 4.2.0 / 1.0.1 | 不照搬旧版 keyring 接入示例 |
| tokio-serial SerialStream 索引 | 5.5.0 | 只作异步串口 API 参考 |

浏览器工具无法打开本轮 russh 0.64.1 的两个固定版本子页面，因此该版本具体方法签名未确认；上文能力参考 latest API/搜索索引，示例不声称适配 0.64.1。M1 必须选择可重复获取的 release/tag 并检查同一版本源代码。没有进行全量漏洞检索或依赖供应链审计。

实施时形成依赖清单：包名、精确解析版本、features、MSRV、来源 tag、许可证、原生编译依赖、已知安全公告、验证平台与结果；提交更新后的 Cargo.lock/pnpm-lock.yaml。避免默认启用 full/all，按需添加 Tokio 功能。允许在开发任务需要时安装/更新依赖并更新锁文件；纯文档预研不要求提前安装。依赖安装不得间接触发项目构建/运行，规则见 AGENTS.md。

许可方面已从仓库确认 xterm.js 为 MIT、russh 为 Apache-2.0、serialport 为 MPL-2.0；其余库与所有传递依赖待锁定版本后核对，发布前生成 notices。特别是 serialport 不能按“所有库都是 MIT”处理。

## 9. 待执行技术验证

| 编号 | 必须回答的问题 | 失败时的处理 |
| --- | --- | --- |
| SP-01 | russh 同一 release 能否完成密钥校验等待/取消、密码/加密私钥、交互认证、PTY 和 resize？ | 调整 release/适配层；不能跳过密钥校验 |
| SP-02 | 三种 WebView 的原始 Channel、解析 ACK、背景标签是否可用且有界？ | 调整帧/批次；二进制不可用时经 ADR 改受界的字节序列化方案 |
| SP-03 | keyring-core 的平台后端能否可靠读写/取消并处理无服务/锁定？ | 收紧支持矩阵或暂禁保存；不降低秘密保护 |
| SP-04 | SQLx SQLite feature、迁移、路径、备份在三平台是否一致？ | 更换版本/隔离存储实现，不暴露 SQL 给前端 |
| SP-05 | 关闭是否打断满队列/无 ACK/远端无响应？KeepAlive 是否仍正常？ | 调整 actor/任务拓扑，未通过不进入 MVP |
| SP-06 | 后续 PTY/串口在设备阻塞时如何唤醒退出？ | 评估专用线程、平台取消 API、tokio-serial |
| SP-07 | Naive UI/xterm 组合能否在三种 WebView 中保持终端跨重连不清空，并正确处理焦点、Portal、CSP 和旧流模式边界？ | 调整控件/流适配层；不能用清屏或重建终端规避 |
| SP-08 | 应用内全局/终端按键与点击手势能否配置、消除冲突/重复执行，并兼容 IME/菜单？ | 调整 router/焦点适配；系统后台另在 M5 验证，不牺牲远端默认控制键 |
| SP-09 | 所选 xterm release 是否映射全部已编辑 palette 字段、可热应用，导入可往返且安全？ | 调整映射或版本，不能静默忽略颜色，不以重建终端热更新 |
| SP-10 | SFTP-only、双栏各端点组合、metadata/冲突/取消/中转与共享连接公平性是否满足？ | 调整 provider/独立连接/能力策略；不跳过身份或覆盖检查 |
| SP-11 | 原生拖入/拖出能否在三平台文件管理器工作，staging 生命周期/冲突是否可靠？ | 替换原生适配、明确受限平台；需求未通过不得声称完整交付 |

SP-01~05、SP-07~09 未执行；SP-10/11 到 M3，SP-06 到 M4，系统后台快捷键到 M5。执行规则和证据记录见 [验证计划](11-validation.md)。

## 10. Reka UI 历史评估（已被 Naive UI 取代）

2026-10-15 按用户对高度自定义桌面 UI 的要求追加官方资料核查，方向从“可选候选”更新为“采用基础控件方案，具体集成待验证”。详见 [UI 选型](13-ui-stack.md)。当前 package.json 仍只有模板依赖，不能说前端已经接入 Reka。

主要理由是无样式、受控状态和组件组合能力，便于建立自己的桌面设计系统；局限是需要自行完成视觉设计、终端保活、拖拽工作区和系统集成。[官方介绍](https://reka-ui.com/docs/overview/introduction) · [样式指南](https://reka-ui.com/docs/guides/styling)

## 11. 快捷键、配色与 SFTP 专项补查

核查日期同本基线；以下是官方 API/维护者资料层面结论，未安装、运行或对服务器实测。新增规格见 [14](14-shortcuts.md)、[15](15-terminal-themes.md)、[16](16-sftp-workspace.md)。

| 资料 | 核查结论 | 不可推断的能力 |
| --- | --- | --- |
| [Tauri Global Shortcut](https://v2.tauri.app/plugin/global-shortcut/) | 提供桌面 OS 全局热键注册能力，可做可选系统后台模式 | 不负责应用 keymap、终端上下文/IME、修饰键+鼠标；不保证 OS 保留组合可注册 |
| [xterm ITheme](https://xtermjs.org/docs/api/terminal/interfaces/itheme/) | 提供基础/光标/选区/ANSI/extendedAnsi/scrollbar 等颜色字段，可建立自有方案模型 | 不负责内置方案、颜色编辑器、第三方主题解析；具体字段需同 release 核验 |
| [russh-sftp SftpSession](https://docs.rs/russh-sftp/latest/russh_sftp/client/struct.SftpSession.html) | open_with_flags、读写/目录/元数据等接口满足 FileProvider 的文档级方向 | set_metadata 不保证任意 uid/gid/权限可修改，rename 不保证所有服务器原子/无覆盖 |
| [RawSftpSession](https://docs.rs/russh-sftp/latest/russh_sftp/client/rawsession/struct.RawSftpSession.html) | 可进一步评估底层协议能力/扩展适配 | 不因有 raw API 就保证某服务器实现 POSIX rename/copy-data/条件替换 |
| [Tauri WebView](https://v2.tauri.app/reference/javascript/api/namespacewebview/) | 原生 drag/drop 事件可用于外部文件拖入授权入口 | 不证明远端虚拟文件拖出 Finder/Explorer 已解决 |
| [drag-rs 仓库](https://github.com/crabnebula-dev/drag-rs) / [说明](https://docs.crabnebula.dev/plugins/drag-rs/) | tauri-plugin-drag / @crabnebula/tauri-plugin-drag 为原生拖出候选 | 未冻结 release、许可与平台支持，不能承诺 file promise/Wayland/未下载远端文件直接拖出 |

远端 A→B 采用两个 SFTP 流在 Rust 有界中转，从 API 能力看可行，但吞吐/内存/取消/共享 transport 公平性都属于 SP-10。原生拖出可先准备真实 staging 文件再开始拖，是否支持更顺畅的延迟文件提供必须经 SP-11，不用 JS 全量文件替代。

链接识别沿用 xterm web-links/显式超链接方向，但本轮未确认 addon 所选 release 的事件回调签名；修饰键手势由应用策略提供，不能把未经核对的方法名写成可直接复制的实现。

## 12. 首期落地版本核查（2026-10-15）

前文为 M0 候选观察，现已落地：russh 0.64.1（Apache-2.0，MSRV1.89，关闭默认features，启用ring+rsa）、Tauri2.11.5/build2.6.3、SQLx0.8.6（MIT OR Apache-2.0，sqlite/runtime-tokio/migrate/macros，关闭默认features）、keyring3.6.3（MIT OR Apache-2.0，MSRV1.75，macOS apple-native、Windows windows-native、Linux sync-secret-service/crypto-rust/vendored）。keyring-core 4.x 是前期方向，本轮由 ADR-017 明确选择 3.6.3 适配器，不混用新文档 API。

已下载并阅读固定 release 源码核对 Handler、认证、PTY/Shell确认、Channel raw Response、宏注册/ACL、SQLx事务和keyring后端。前端 xterm6.0.0、Fit0.11.0、Search0.16.0、WebLinks0.12.0、Reka2.11.0、Pinia4.1.0，以锁文件和包内类型为准，安装禁用scripts。xterm/Reka/Pinia主包许可证MIT；传递notice清单待发布阶段完整生成。

pnpm audit报告0个已知公告；OSV对688个Rust锁依赖查询发现RSA/GLib/停止维护条目，影响路径和未关闭门槛见 [实施状态](17-implementation-status.md#4-依赖与公告核查)。锁文件还含未启用的SQLx/MySQL可选依赖，按cargo tree启用features判断实际可达性。未编译或运行三平台，MSRV声明不等于完整组合已通过。

启动反馈修正：2026-10-15，JS 的 @tauri-apps/api 不再使用宽泛 ^2，固定 2.11.0 与 Rust tauri2.11.5 同 minor。未使用的 JS plugin-opener 已移除，原生 opener2.5.5 通过受控命令保留。pnpm 锁文件同步，未升级原生栈或主动启动项目。

## 13. Naive UI 固定版本核查（2026-10-15）

按用户最新要求用 naive-ui 2.45.3 替换 reka-ui。已核对注册信息与本地固定包：MIT，Node >=20，Vue peer ^3.0.0，无 Rust features/MSRV 影响；NModal/NSelect/NInput/NInputNumber/NTabs/NColorPicker 等 API 以包内声明和实现为准。依赖按需静态导入，不新增图标库或 Tailwind。执行 pnpm add --save-exact naive-ui@2.45.3 --ignore-scripts，删除依赖使用禁用脚本配置；锁文件同步。

本轮 pnpm audit --json 返回 0 个已知公告；不代表没有供应链风险，也不关闭既有 Rust 公告。未执行构建、启动、性能或三平台兼容性验证，未宣称包体积收益。来源与控件分工见 [UI 选型](13-ui-stack.md)。此前第10节及首期依赖记录是历史证据，当前依赖以 package.json/pnpm-lock.yaml 为准。


## 14. 凭据材料封装依赖（2026-10-15）

terminal-security 复用锁文件已有 serde 1.0.228（derive）、serde_json 1.0.154 和 uuid 1.22.0（v4），使用精确版本声明；分别用于私钥材料的单项 JSON 系统凭据封装及新项目 ID，不更换 keyring 后端。锁文件同步直接依赖，无新增未核对的库版本。三者许可为 MIT OR Apache-2.0；serde 声明 MSRV 1.56、serde_json 1.71、uuid 1.85，未提高既有 russh 1.89 基线。敏感材料使用 Zeroizing，接口不序列化返回材料。仅检查锁文件解析与源码；三平台系统凭据单项容量和完整依赖组合的编译/运行兼容性待验证。

## 15. 内置终端字体核查（2026-10-15）

从 DejaVu、Mozilla、Adobe、Microsoft、Fira Code、JetBrains 官方 GitHub 的 latest 正式 Release、Canonical 官方字体包、Meslo 作者仓库和 AOSP 官方镜像获取字体；Anonymous Pro 与 PT Mono 采用 Google Fonts 官方固定分发提交。详情见 [字体来源清单](../public/fonts/README.md) 和 [含 SHA-256 的 manifest](../public/fonts/manifest.json)。未引入运行时字体下载服务或额外 npm/Rust 依赖，下载后用 fontTools 静态读取字体表和内部版本，不执行应用。采用原始字体文件，未重打包转换/子集化。许可分别为 OFL、Ubuntu Font Licence、Apache-2.0、Bitstream Vera/DejaVu，逐家族附原许可与版权。官方站点不能完整验证的分发情况已单独记录，不将仓库 head 说成独立发布版本。

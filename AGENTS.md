# Termeta 开发协作指南

更新：2026-10-17。适用于整个仓库，文档和代码注释默认中文。本文维护协作规则与关键架构约束；功能状态以 [产品需求](docs/01-product-requirements.md#8-当前实现与限制) 为准，不追加逐轮修复日志。

## 1. 项目定位与范围

Termeta 是 Tauri 2 + Rust + Vue 3 + TypeScript 桌面终端客户端，目标平台为 Windows、macOS、Linux，不计划 Android/iOS。以 Termius 桌面核心体验为功能参照，不复制品牌或专有实现；用户指定的 SSH 系统/类型图标素材是明确例外。

已有 SSH、Telnet、基础 Local/Serial 文本终端、加密保险库、主机/分组管理、分屏广播、显式工作区存档、SFTP 和本地/远程/SOCKS5 转发源码。当前版本未完成完整集成、三平台和发布验收，不能标为 MVP 已验收。

## 2. 重要执行约束

- **不要主动构建、运行或启动项目。** 未得到用户明确要求时，不执行 `pnpm dev`、`pnpm build`、`pnpm preview`、`pnpm tauri dev/build`、`cargo build/run/check/test/clippy`，也不通过 IDE、脚本、测试、CI 或其他命令间接触发编译、启动、打包。`cargo check`、Rust 测试同样会编译，不能作为绕过方式。
- **允许按当前任务需要主动安装/更新依赖库、下载必要工具、更新锁文件，以及执行不构建/运行项目的静态检查，无需为这些操作单独确认。** 安装若会自动触发项目构建或运行，先禁用相关生命周期脚本，不能借依赖安装绕过上一条限制。
- 不主动启动 SSH 测试服务器、连接真实主机或串口；这些运行验证在用户明确要求的范围内执行。
- 可进行源文件阅读、联网查阅官方资料、差异检查、Markdown 链接检查等不会构建/运行项目的静态检查。报告中明确区分“静态检查”与“编译/运行验证”；未验证就写未验证。
- 仅编写文档时不为示例提前创建空 crate、添加无关依赖或改写模板业务代码；开发任务所需的依赖安装按上面的允许规则执行。
- 用户已有修改不得覆盖；不要自行提交、推送、发布或创建真实凭据。
- **已经实际应用的数据库迁移（包括开发环境）必须保持原始字节不变。** 后续修改通过新增迁移完成，不跳过 SQLx 校验、不手工改写 `_sqlx_migrations` 校验和；迁移故障先保留加密数据库及配套 vault 元数据。

## 3. 当前真实状态

| 领域         | 当前范围与限制                                                                                                                                       |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| 主机与保险箱 | SSH/Telnet/串口和工作区四类保存项；分组、收藏、标签、拼音/字段搜索、侧栏显示开关、卡片/列表、拖拽与批量打开。四类名称 trim 后区分大小写、全库唯一    |
| 终端         | SSH 密码/公钥/动态 OTP、主机信任、PTY/resize/KeepAlive、Shell/命令启动；Telnet 明文终端登录；本地系统 Shell；串口参数/字符集/独占。失败/断线保留视图和历史，手动重连 |
| 工作区       | 顶部/左侧标签、嵌套分屏、窗格迁移、比例调整/区域交换、输入广播。显式保存布局与已保存 SSH/Telnet/串口引用，重新打开创建新会话，不自动恢复标签         |
| 文件与转发   | 单 SSH 终端 SFTP 侧栏、双远端对拷、原生拖入及 macOS 文件/目录承诺拖出；独立三模式端口转发。通用租约、本地浏览、Windows/Linux 拖出待实现              |
| 设置与存储   | 独立设置窗口、有效修改自动保存/跨窗口热应用、应用明暗与主色、简体中文/English/跟随系统、快捷键、12字体/64配色。SQLCipher 加密格式2、schema 6，同格式 schema 2–5 原位升级；设置安全页可校验旧密码并重新加密改密；Gist/WebDAV 手动加密备份、非覆盖合并及首次云端初始化        |

数据库及配套 vault 元数据须配对保留，OS 只缓存一个 `master-key-v2` 主密钥；密码恢复不回退明文。旧格式不迁移。已应用迁移不可变；迁移4保持原始字节，名称索引调整由迁移5完成，恢复说明见 [存储设计](docs/07-storage.md#9-已应用迁移校验和不一致的恢复)。

已有 macOS 用户反馈；历史授权调试中 terminal-core/terminal-serial 定向编译和15项测试通过，指定 ESP32-C3 的115200输出/关闭已验证。该证据不覆盖桌面应用、后续持久化集成或其他平台。其他新增 Rust 回归源码不能视为已执行。Telnet 手动测试服务配置见 [dev/telnet](dev/telnet/README.md)，既有部署/就绪证据不代表服务当前仍运行，也不授权构建或启动应用。

主要待实现范围：专门睡眠恢复、本地自定义 Shell/cwd/env 与完整进程树清理、串口手动 DTR/RTS/DSR-DTR/HEX、ProxyJump/Agent、完整传输管理、Snippets/会话日志、打开标签自动恢复、自动备份/双向同步、本地备份文件入口与签名发布。

## 4. 架构约定

- `src/layout/` 负责保险箱装配与外观监听；App 保留根布局与稳定终端宿主。设置由独立 SettingsWindow 装配，公共基础样式位于 `src/assets/base.css`。
- `src/modules/` 按 hosts、credentials、history、forwarding、terminal、workspace、settings、files、backup 分责；Vue/Pinia 只保存轻量状态，不保存终端输出、文件正文、数据库备份正文或转发载荷。
- `src/services/desktop/` 是唯一 Tauri IPC 门面；组件不能直接 invoke。CommandRegistry 统一按钮/菜单/快捷键；KeymapService/TerminalThemeService 是设计职责名，不假定存在同名类。
- `src-tauri/src/commands/` 负责 DTO、窗口/资源所有权、Channel 和编排；`files/` 负责浏览器及上传下载服务；协议核心留在 crate。
- 九个领域 crate 为 core、ssh、telnet、local、serial、storage、security、files、sftp。core 不依赖 Tauri/Vue/russh；具体适配器依赖 core。security 当前直接依赖 storage 协调保险库、凭据事务和备份压缩/加密；云端 provider 位于壳的 backup 模块，新增存储厂商通过 provider 边界扩展。
- 每会话 actor + 有界字节队列 + 独立取消信号；输出二进制 Channel，控制类型化 Channel，消费 ACK 实现端到端背压。所有凭据、信任、网络与设备操作由 Rust 管理。
- terminalId 跨重连稳定，sessionId 每次连接更新；TerminalRegistry 按 terminalId 持有 xterm。断线/失败/终态不得自动关闭 Tab、清空或销毁实例，只有用户关闭视图才 dispose。
- WorkspaceStore 分离终端 tabs 与标签 groups；activeId 指组，active 指聚焦终端。布局树计算矩形，TerminalStage 扁平 keyed 宿主保证排序、交换、分屏与迁移不重建终端。
- 终端、文件和转发分别使用 sessions/file_sessions/forward_sessions；文件与转发独立连接不申请 PTY/Shell。通用跨功能连接租约和完整 TransferManager 尚未实现。

## 5. 文档索引与阅读顺序

开发前至少阅读本文、目标模块文档及 [路线](docs/10-roadmap.md)。完整导航见 [docs/README.md](docs/README.md)。

| 文档                                                                                                           | 用途                                         |
| -------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| [产品需求](docs/01-product-requirements.md)                                                                    | 源码完成勾选、入口、限制、依赖公告与验收范围 |
| [架构](docs/03-architecture.md)、[会话](docs/04-session-model.md)、[IPC](docs/06-ipc-and-streaming.md)         | 模块边界、生命周期与数据流                   |
| [存储](docs/07-storage.md)、[安全](docs/08-security.md)                                                        | 迁移、凭据一致性、ACL 与安全边界             |
| [桌面交互](docs/09-desktop-ux.md)、[UI](docs/13-ui-stack.md)                                                   | 页面、组件、主题、焦点及表单                 |
| [快捷键](docs/14-shortcuts.md)、[配色](docs/15-terminal-themes.md)                                             | 输入路由、热应用与资源                       |
| [SSH](docs/05-ssh-design.md)、[SFTP](docs/16-sftp-workspace.md)、[工作区](docs/18-terminal-workspaces.md)      | 功能契约及资源边界                           |
| [Serial](docs/19-serial-terminal.md)、[Telnet](docs/20-telnet-terminal.md)、[转发](docs/21-port-forwarding.md) | 协议/设备与连接工具                          |
| [备份同步](docs/22-data-backup.md) | 白名单 SQLite 快照、加密包、云端 provider 与非覆盖导入 |
| [系统终端调研](docs/23-terminal-system-integration-research.md)、[实现方案](docs/24-terminal-system-integration-plan.md) | 三平台默认终端接口、启动本地标签与平台集成计划（待实现） |
| [有效决策](docs/12-decisions.md)、[依赖预研](docs/02-library-research.md)                                      | 理由、变更条件与版本来源                     |

## 6. 修改后的文档同步义务

1. 功能、架构、接口、数据、安全、依赖或范围变化时，同轮更新需求状态、路线剩余门槛及相关模块文档；必要时同步本文摘要。
2. 关键决策变更更新决策文档并说明日期、理由和影响；接口冲突须修订契约与 Rust/TS 类型，不临时创建第二套接口。
3. 增删文档同步索引和本地链接。文档保留最终状态、有效约束与必要验证证据，不重复粘贴实现日志、微小尺寸变化和逐轮静态检查记录。
4. `[x]` 只表示完整源码实现；部分实现保持 `[ ]` 并说明剩余范围。不能把计划或静态通过写成运行验收通过。
5. 交付报告说明修改、实际静态检查及未执行的编译/运行验证，不为满足清单擅自执行命令。

## 7. 实现基本规则

- NForm 的 `model` 必须绑定共享响应式对象，控件 `v-model` 和 `path` 指向同一字段；禁止用模板对象字面量、对象展开或 `computed(() => ({ ... }))` 生成表单值快照。已有 ref 可通过 `reactive({ fieldRef })` 保留同一字段引用，不能复制 `.value` 后异步同步。字段嵌套调整须同步 rules/path；系统文件选择、重置按钮等程序更新入口应通过 FormItem 的公开 validate API 重验对应字段。保留正常 input/change/blur 和提交校验，不用取消校验、选中后盲目清错或 nextTick 补丁掩盖旧值问题。`pnpm check:structure` 检查临时/派生 model 的常见写法。

- 通用 UI 图标统一从 `@vicons/ionicons5` 按名称导入并通过 Naive UI `NIcon` 承载，不新增手写 SVG 或字符替代图标；SFTP 文件类型及主机分组文件夹保留现有自绘 SVG。图标按钮保留可访问名称，装饰图标使用 `aria-hidden`；系统原生窗口图标和桌面应用品牌资源不属于 UI 图标替换范围。用户指定的28个 SSH 系统/类型 SVG 为明确例外，来源及摘要见 `resources/host-system-icons.json`；`AssetIcon.vue` 只做语义名到 Ionicons 的薄映射。

- Vue 文件统一使用仓库 Prettier 配置，`htmlWhitespaceSensitivity: "ignore"` 仅对 `*.vue` 生效。编写模板时保留清晰的空格、缩进和换行，多行元素的正文、完整闭合标签和相邻组件分别换行，避免 `文字</AppButton` 换行后接 `><AppButton` 的紧贴写法；短元素允许格式化为单行。修改后执行 `pnpm format:vue` 自动整理、`pnpm check:format:vue` 静态校验，均不构建或运行项目。此规则不承诺保留行内元素边界的空白语义；必须精确保留的文本空格使用显式文本表达式，必要时对单个节点使用 `<!-- prettier-ignore -->` 并说明原因，保留 `pre` 等预格式化内容。
- **写代码时使用中文注释。** 新增或修改的行内注释、块注释和文档注释均使用中文；标识符、API 名称和必要的技术术语保持原文。注释说明意图、边界与约束，避免重复代码含义。
- 界面新增文案使用 `src/i18n` 的 t 与模块词典；动态整句使用命名参数，选项/校验展示保持响应式。用户内容与远端输出保留原文，提示面向操作和后果，不展示内部实现说明；执行 `pnpm check:i18n` 静态校验。
- 前端 pnpm；沿用 Vue 3 Composition API、严格 TypeScript。Rust 当前 edition 2024。暂不切 React，也不引入 WebSocket/HTTP 后端。
- 禁止默认接受所有 SSH 主机密钥、明文保存密码/私钥、默认开启弱算法、自动执行远端输出、自动重放断线前输入。
- 自定义命令必须纳入 `AppManifest::commands` 对应 ACL 并校验调用窗口及资源所有权；只写 capability JSON 不足以保护默认自定义命令。
- 不在全局锁内进行网络/数据库/Channel 等待；不能让输出 ACK 堵塞会话关闭、KeepAlive 或信任响应。
- 后续依赖落地时重新核对具体 release 的 API、features、MSRV、许可证、安全公告，更新锁文件和预研；`latest` 页面不是版本锁定策略。

## 8. 交互实现约束

- 通用控件优先 Naive UI 2.45.3，工作区标签由自定义 WorkspaceTabs 管理；封装只做薄适配，使用库主题/公开 props/themeOverrides，不重造控件皮肤。业务 CSS 就近位于 Vue scoped 样式，跨组件动态 DOM 使用明确 `:deep()` 或入口全局适配。
- NForm model/rules/path 与控件共享响应式字段，提交前 validate，必填星号由库生成；快速连接名称、可选密码和服务器允许的空回答不强制必填。弹窗操作放 AppDialog `#footer`，与滚动正文分离。
- 主机页保活显隐挂原生 DOM，不能直接给观察器为根的滚动组件施加 v-show。分组树/主机列表独立滚动；即时偏好串行局部保存，设置旧草稿不能覆盖。搜索不改变分组范围，分组批量打开包含后代。
- 分组迁移前后端防环；删除默认保留直属主机/子组并上移，连同内容删除须独立二次确认。Rust 单事务保护范围和 revision，共享凭据与现有会话快照保留。
- 固定保险箱/SFTP 不可关闭、不创建终端；切页与标签位置变化不卸载 xterm。设置独立单实例窗口，settings capability 只开放设置、专用保险库改密/JSON 导出和备份同步命令，通用会话 owner 仍仅接受 main。
- 分隔条与交换手柄仅悬停显示；交换完整子树、保留比例和终端，不改变广播成员。交换按钮不获取焦点、不触发拖动，分隔条键盘调整保留。
- 广播仅内存、默认关闭；成员重组关闭，排序/比例调整/区域交换不关闭。inputOrigin.ts 集中适配固定 xterm 6 用户输入标记及最终鼠标动作编码，能力缺失禁止开启广播。远端滚轮仅广播源端最终输入，不派发合成 wheel 或让目标重新累积；当前组未聚焦窗格也可作为滚轮来源，键盘/粘贴仍受焦点约束。排除命令模式窗格、点击、拖动、设备应答及认证；各目标独立编码/代次检查，不重放。
- 主机/分组/终端右键菜单共用外部关闭过滤，忽略右键及 macOS Control+单击的打开手势；弹层标记随实际显示状态更新。连接/认证弹窗 after-leave 后恢复当前活动终端焦点。模态、录制与弹层优先处理快捷键，弹窗内 Cmd+W 不关闭标签或主窗口。
- 标准粘贴快捷键放行系统 paste，由终端宿主执行多行确认/大小/广播校验；自定义绑定保留显式读取剪贴板路径。应用内快捷键与系统后台热键分开。
- SFTP 仅单 SSH 终端提供侧栏，开启时禁止作为分屏来源或目标；隐藏保留任务，断开/关闭取消，重连不重放。双栏独立连接与对拷，不使用原生拖出处理内部复制。
- 文件正文仅 Rust 流动；上传普通文件确认覆盖、目录合并，不承诺回滚；下载无覆盖发布。原生拖入只消费 owner-bound 一次性 dropId，不接受前端任意本地路径；macOS 原生承诺请求兑现后才显示任务，取消清理手势与迟到请求。限额及细节见 [SFTP](docs/16-sftp-workspace.md)。
- SSH 主要认证为密码或公钥；Keyboard-interactive/OTP 按服务器协商，OTP 不保存。保存的交互 Shell 模式 SSH 终端的一次性系统识别使用固定只读命令与独立 exec，不写 PTY/广播。
- 端口转发默认回环监听，SOCKS5 只允许回环且仅 TCP CONNECT；独立取消与窗口/心跳清理，不持久化运行状态或自动启动。
- 云备份仅包含主机/分组/标签/工作区、凭据、转发规则和主机信任的白名单快照；应用设置、历史与云端凭据留在本机；备份密码随Token加密保存在本机settings.backup，读回仅返回存在标记，操作直接使用。快照先压缩，再与时间/设备名一起认证加密；上传创建独立版本，下载预览后仅新增合并，不覆盖本机。首次云端初始化仅允许全新保险库，业务导入与本机云配置同事务；锁定的现有库不得使用初始化入口。
- 应用主色与终端配色独立；终端热应用不清历史、不重连。字体仅限内置12家族；资源来源与许可见 public/fonts 和 resources。

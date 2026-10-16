# Termvexa 开发协作指南

最后更新：2026-10-16。适用于整个仓库。文档语言默认中文。

## 1. 项目定位与范围

Termvexa 是基于 Tauri 2 + Rust + Vue 3 + TypeScript 的桌面终端客户端，长期以 Termius 的桌面核心体验为功能参照。当前实现 SSH 与基础 Local Terminal，架构保留 Serial、SFTP、隧道扩展。仅支持 Windows、macOS、Linux；Android、iOS 不在计划内。对标功能与体验，不复制品牌、素材或专有实现。

## 2. 重要执行约束

- **不要主动构建、运行或启动项目。** 未得到用户明确要求时，不执行 `pnpm dev`、`pnpm build`、`pnpm preview`、`pnpm tauri dev/build`、`cargo build/run/check/test/clippy`，也不通过 IDE、脚本、测试、CI 或其他命令间接触发编译、启动、打包。`cargo check`、Rust 测试同样会编译，不能作为绕过方式。
- **允许按当前任务需要主动安装/更新依赖库、下载必要工具、更新锁文件，以及执行不构建/运行项目的静态检查，无需为这些操作单独确认。** 安装若会自动触发项目构建或运行，先禁用相关生命周期脚本，不能借依赖安装绕过上一条限制。
- 不主动启动 SSH 测试服务器、连接真实主机或串口；这些运行验证在用户明确要求的范围内执行。
- 可进行源文件阅读、联网查阅官方资料、差异检查、Markdown 链接检查等不会构建/运行项目的静态检查。报告中明确区分“静态检查”与“编译/运行验证”；未验证就写未验证。
- 仅编写文档时不为示例提前创建空 crate、添加无关依赖或改写模板业务代码；开发任务所需的依赖安装按上面的允许规则执行。
- 用户已有修改不得覆盖；不要自行提交、推送、发布或创建真实凭据。

## 3. 当前真实状态

M1/M2 SSH 主体源码已落地，处于体验修复与集成验收阶段，不能标为 MVP 已验收。用户有 macOS 编译和终端反馈，最新源码及三平台完整矩阵仍待验收。

| 模块       | 已有源码                                                                                                                                                                                                  |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 主机与凭据 | 主机/左侧文件夹分组树（含子组主机计数、展开/包含子组偏好入库、顶部独立搜索下拉（拼音开关/字段选择即时入库）、收藏置顶、右键增改删、分组/主机拖拽迁移、删除保留上移或二次确认整树删除）/收藏/标签/搜索（切回保险箱默认主机页并自动聚焦搜索框）、全局新建标签页弹窗（共用主页拼音/字段搜索，结果内嵌，快速连接入口，＋/Cmd+T，旧Cmd+N同入口）、精简主机卡片（操作集中于右键菜单，SSH 标签紧随名称居中）、公共密码/私钥、主机专属密码、AES-256加密数据库/单主密钥钥匙串                               |
| SSH 与终端 | 密码/公钥/动态 OTP（明确 PAM 密码提示自动使用保存密码一次，空白零输入轮自动应答）、主机信任、PTY/resize/KeepAlive、二进制 Channel/ACK、有界输入输出、断线保留与原终端重连                                 |
| 工作区     | 固定保险箱/SFTP、标签栏集成窗口标题栏、顶部32px/左侧36px标签（顶部＋为32px正方形，侧栏＋横向填满）、排序/嵌套分屏/窗格迁移、比例调整、组内输入广播、远端标题展示及 OSC 133/标题冒号前缀辅助关闭确认；普通标签复制/拆分、可配置双击动作，Workspace 标签无右键菜单                |
| 设置与 UI  | Naive UI 2.45.3、组件内 scoped 样式、独立设置窗口与自动应用（标签位置单选、外观图标单选、常规选项统一单列宽度，默认配色仅在终端配色页设置）、快捷键、12种本地字体（默认 DejaVu Sans Mono）、64套终端配色（默认 Pro）、JSON 导入导出与热应用 |
| SFTP       | 单 SSH 终端可调右侧栏、分页及链接浏览、普通文件上传下载、目录拖入合并、文件覆盖确认、macOS文件/目录承诺拖出、独立松键检测与WebView鼠标收尾、进度与取消；仅浏览上传下载                                    |

固定 SFTP 双栏已支持从保存主机建立两条独立 SSH/SFTP 连接、Web 栏间文件/目录对拷；不启动 PTY/Shell，本轮待编译运行验收。Windows/Linux 原生拖出、通用连接租约、完整批量冲突策略、隧道/跳板/Agent、Serial、整体 Workspace 合并、布局持久化和签名发布尚未实现。用户已确认 macOS 拖动取消后首次点击和再次拖动恢复正常；原生拖放完整矩阵仍待验收。双栏对拷经Rust以64 KiB中转，文件覆盖逐项确认、目录只合并，Pointer Events不依赖系统拖动。功能需求表的 `[x]` 仅表示代码已实现，不考虑测试；未实现或部分实现保留 `[ ]` 并说明剩余范围。源码入口及验证边界统一见 [需求与实现状态](docs/01-product-requirements.md#8-当前实现与限制)。

连接历史仅保留最近20次（SSH/Local合计，不按主机去重），新增事务内裁剪、解锁清理已有超量记录，不影响终端滚屏历史；源码待运行验收。

当前使用 `termvexa-encrypted-v2.sqlite3`、schema 2，全库由SQLCipher 4以AES-256加密；配套 `termvexa-encrypted-v2.vault.json` 保存随机盐、Argon2id参数和GCM密钥校验。首次启动设置至少12字符密码，Argon2id（64MiB/3轮/1并行度）派生32字节主密钥；OS钥匙串只缓存一个 `master-key-v2` 项。后续每进程自动读取一次，失败时输入密码恢复并重写OS主密钥；缓存写失败允许本次解锁并明确提示。未解锁拒绝业务IPC与数据库操作。新格式不迁移旧库/旧OS凭据，源码与完整性/恢复测试待编译运行验收。主机直接密码非空自动保存、显式空值移除、省略保留；私钥正文与可选解密密码整体存入加密数据库，不依赖原文件，凭据接口不返回秘密。

## 4. 架构约定

当前源码已按职责提取保险箱布局、终端关闭/菜单、主机分组拖动及侧栏传输队列，目录展示规则由两种 SFTP 视图共享；仅静态验证，交互与三平台仍待验收。

当前已建立 `src/modules/`、`src/services/desktop/`、`src-tauri/src/commands/` 及 core/ssh/local/storage/security/files/sftp 七个 crate。完整目标结构见 [架构文档](docs/03-architecture.md)，后续领域仍是计划。

- `src/layout/`：保险箱布局与应用外观监听；App 保留根装配和稳定终端宿主。主机/设置页面为 `HostsPage.vue` / `SettingsPage.vue`；固定双栏 `SftpWorkspace.vue` 属于 files 模块。
- `src-tauri/src/files/`：壳内 SFTP 浏览器获取、上传与下载服务，commands 仅承担入口校验、授权和编排；`terminal-sftp` 分为 browser/remote_file/copy，公开类型路径不变。
- `src/modules/*`：hosts、credentials、terminal、workspace、settings、files；完整 transfers 服务仍为后续；Vue/Pinia 只保存轻量状态，不保存终端输出或文件正文字节。CommandRegistry 统一动作与快捷键；终端配色优先级见 terminal/registry.ts，转换见 settings/themes.ts，选区合成薄适配见 terminal/theme.ts。KeymapService/TerminalThemeService 为设计职责名，不能假定存在同名类。
- `src/services/desktop/`：唯一 Tauri IPC 门面；组件不直接调用 `invoke`。
- `src-tauri/src/`：命令 DTO、窗口所有权校验、Channel 桥接、依赖组装；不承载协议核心逻辑。
- `crates/terminal-core/`：不依赖 Tauri/Vue/russh 的会话模型、能力、actor 控制、生命周期和扩展协议。
- `crates/terminal-ssh/`：russh、主机密钥校验、认证、PTY/Shell、KeepAlive；依赖 core，通过接口使用凭据与信任仓库。
- `crates/terminal-storage/`、`crates/terminal-security/`：SQLCipher全库加密与主密钥派生/系统钥匙串缓存适配器。security 当前直接依赖 storage 协调凭据事务，core 不依赖二者，壳负责注入。
- Local 已由 terminal-local/portable-pty 工厂实现；Serial 的工厂与配置扩展点保留。SFTP/隧道通过 SSH 连接服务扩展，不塞进通用终端接口。`terminal-files` 已定义首批文件元数据/provider/受界传输端口，`terminal-sftp` 经 owner-bound `SshChannels` 复用已认证连接；侧栏随终端生命周期，固定双栏另由文件专用 SessionManager 管理两条独立连接。通用连接租约、完整冲突/任务管理和本地浏览仍是 M3 后续；本地浏览不依赖 Local PTY，独立 SFTP 不启动 Shell。

会话采用**每会话 actor + 有界字节队列 + 独立取消信号**；重连创建新的后端 session ID，但保留同一 Tab、terminalId 与 xterm 实例，在原缓冲中追加输出。断线、连接失败和后端终态均不得自动关闭 Tab、清空或销毁终端；历史继续可滚动、选择、复制和搜索。TerminalRegistry 按 terminalId 持有实例，不能以 sessionId 变化触发重建。输出走二进制 Channel，控制事件走类型化 Channel，使用消费 ACK 实现端到端背压。所有凭据、信任与网络操作由 Rust 管理。

顶部 app-tabs 承载窗口标题栏；macOS 使用 Overlay 并保留原生红黄绿按钮，按钮已下移以对齐54px栏中线（待运行复验），Windows/Linux 使用无系统标题栏与库按钮，空白区域拖动窗口。左侧标签模式保留 54px 顶部窗口栏；关闭按钮沿用会话/传输确认。三平台窗口操作待运行验收。

保险箱和 SFTP 为不可关闭的固定 Tab，SSH 标签随后；设置 `tabPosition` 控制顶部/左侧布局，旧设置默认顶部；`tabDoubleClickAction` 保存双击动作，默认无动作。主机/分组/终端手动右键菜单共用外部关闭过滤，忽略右键及macOS Control+单击的outside回调，避免保活菜单在打开手势松键时误关；主机/终端菜单仅在显示时持有弹层标记，连接/认证弹窗after-leave后恢复活动终端焦点；应用快捷键在普通页面输入框及主机搜索下拉中仍可消费，终端动作仅在终端中生效；模态弹窗、录制控件及其他弹层优先拦截；弹窗内Cmd+W显式消费，不关闭弹窗/标签/窗口，macOS原生Close Window菜单不注册Cmd+W，避免绕过前端路由。普通终端标签可复制为独立连接或向右拆分为 Workspace，开启 SFTP 侧栏时禁止拆分；Workspace 标签无右键菜单，窗格菜单保留，双击仅支持关闭。复制沿用连接目标与原始名称，不复制会话历史。保险箱内侧栏为主机、凭据、主机信任（Known Hosts）、连接历史；设置由应用菜单打开独立单实例窗口。固定页面不创建 session/xterm，切换页面与布局不卸载终端。固定 SFTP Tab 为独立双远端文件工作区；单 SSH 终端另有附属侧栏，详见 [SFTP设计](docs/16-sftp-workspace.md)。认证按服务器协商自动补充 Keyboard-interactive，不由用户配置追加 OTP。仅对单个非回显且明确为 Password/密码的提示自动使用配置中的保存密码一次；失败重试、OTP、未知或复合提示由用户输入。无提示且名称/说明均为空白的轮次直接空响应，不创建弹窗；非空通知保留。自动响应与人工交互共用提示限额和八轮预算。

业务输入表单统一使用 Naive UI NForm/NFormItem 的 model、rules 和提交前 validate，必填星号由库生成；快速连接名称、可选密码与服务器定义可为空的认证回答不强制必填。

前端统一采用 **Naive UI 2.45.3**，工作区顶部/侧栏标签按用户要求使用自定义 WorkspaceTabs，支持拖拽排序/合并，标签组与稳定 terminalId 分离；其余所有有库对应能力的交互控件优先直接使用 Naive UI；项目封装只做薄适配。使用 NConfigProvider/库主题与现成样式，不另造控件皮肤，仅保留业务布局、尺寸和终端必要 CSS。组件业务 CSS 就近写在对应 Vue 的 `<style scoped>`；src/assets/base.css 供两个入口共享主题/基础规则、共享布局和 xterm/传送菜单的全局适配，字体资源独立保留。跨组件动态 DOM 使用明确的 `:deep()` 或入口全局规则，不重建集中式业务 styles.css。终端宿主与工作区生命周期由应用掌控，Tab 显隐不得销毁 xterm。系统文件授权与 xterm 不由 UI 库替代。三平台集成尚未验证，详见 UI 选型文档。

保险箱主机页保活显隐必须挂在原生 DOM 容器，不能直接给以观察器为根的滚动组件施加 v-show；主机页左右分组树/主机列表各用独立 NScrollbar，凭据页另用 NScrollbar，设置独立窗口占满可用高度；凭据与设置页使用 NH1/NText 页头，主机页移除 Header，搜索框置顶，其下左对齐新建主机/本地终端按钮；三页沿用统一页面间距，不显示 eyebrow 或保险箱品牌/页脚装饰。自定义配色字段行填满表单宽度，取色器由可伸缩原生容器承载，重置按钮不收缩；基础色、ANSI色和滚动条色均以NGrid按容器宽度单列/双列排布，视觉待运行复验。配色选项以 NGrid 自适应平铺，网格自然展开，不限高且不设独立滚动，随设置正文统一滚动；搜索/导入位于上方，自定义内容用 NCollapse 默认折叠在网格下方，编辑内置配色自动生成自定义副本，有效编辑自动保存并应用。点击卡片立即选择并自动持久化全局配色；展开自定义后才初始化预览。内嵌 AppDialog 将 NScrollbar 正文和底部 footer 分开，设置自动应用状态及错误重试区始终可见，不提供显式保存/放弃按钮。弹窗操作使用 AppDialog 的 `#footer` 并转发到 NModal card footer，正文由库的 content-scrollable 管理，按钮不能放进滚动正文。界面装饰文字默认不可选，输入/编辑区、终端自身选区和显式可复制正文保留选择能力。

快捷键设置分应用内全局与终端类别（含修饰键点击链接）；系统后台热键另行 opt-in。Cmd+L（Windows/Linux 为 Ctrl+Shift+L）的 Local 动作已接入本地工厂。终端字体固定为12个随应用分发的家族，默认 DejaVu Sans Mono；字体、来源清单和许可证位于 `public/fonts/`，CSS 别名与异步字宽重算避免系统字体版本差异；旧自由字体串读取时归一为默认项，保存由 Rust 白名单校验。终端配色独立于应用主题，内置64套从用户指定解包数据静态提取的方案，默认 Pro；前后端共用 `resources/terminal-palettes.json`，提取脚本与来源摘要随仓库保留；旧自有预设已移除，旧明暗引用读取时分别归一至 Nord Light/Dark，同名 Ocean 使用提取值，用户自定义方案保留。配色支持任意取色与 JSON 导入，热切换不清历史。SFTP 提供终端右侧栏与固定双远端工作区，范围为浏览、上传下载和对拷；上传覆盖须逐项确认，目录始终合并。原生拖出须单独验证，不能用 Web DnD 替代。

## 5. 文档索引与阅读顺序

| 文档                                                            | 用途                                                  |
| --------------------------------------------------------------- | ----------------------------------------------------- |
| [docs/README.md](docs/README.md)                                | 基线、术语、文档导航                                  |
| [01-product-requirements.md](docs/01-product-requirements.md)   | 需求完成勾选、源码状态、限制、公告与验收                              |
| [02-library-research.md](docs/02-library-research.md)           | 联网预研、来源、版本观察与待验证项                    |
| [03-architecture.md](docs/03-architecture.md)                   | 分层、依赖、目标目录、扩展方式                        |
| [04-session-model.md](docs/04-session-model.md)                 | actor、状态机、trait、清理语义                        |
| [05-ssh-design.md](docs/05-ssh-design.md)                       | SSH 握手、信任、认证、PTY、异常                       |
| [06-ipc-and-streaming.md](docs/06-ipc-and-streaming.md)         | 命令、DTO、二进制帧、顺序和背压                       |
| [07-storage.md](docs/07-storage.md)                             | 表结构、迁移、凭据引用与一致性                        |
| [08-security.md](docs/08-security.md)                           | 威胁模型、ACL、CSP、安全默认值                        |
| [09-desktop-ux.md](docs/09-desktop-ux.md)                       | 主机管理、标签、终端生命周期、桌面交互                |
| [10-roadmap.md](docs/10-roadmap.md)                             | 里程碑、任务依赖、风险、进度                          |
| [12-decisions.md](docs/12-decisions.md)                         | 架构决策与变更条件                                    |
| [13-ui-stack.md](docs/13-ui-stack.md)                           | Naive UI 固定版本、控件映射、主题与桌面集成边界       |
| [14-shortcuts.md](docs/14-shortcuts.md)                         | 全局/终端快捷键、修饰键点击、冲突/焦点/配置与原生边界 |
| [15-terminal-themes.md](docs/15-terminal-themes.md)             | 内置/自定义配色、取色编辑、导入导出与热应用           |
| [16-sftp-workspace.md](docs/16-sftp-workspace.md)               | SFTP 浏览、上传下载、拖放与安全边界                   |
| [18-terminal-workspaces.md](docs/18-terminal-workspaces.md)     | 标签排序、嵌套分屏、输入广播、焦点与保活边界          |

开发前至少阅读本文、目标模块文档及路线。接口冲突应修订文档与实现，不临时增加第二套契约。

## 6. 修改后的文档同步义务

每次实现功能、修改架构、接口、数据结构、安全策略、依赖选型或开发范围，**在同一轮修改中**：

1. 更新本文件的“当前真实状态”、架构说明与必要索引；不要把计划标成完成。
2. 更新 `docs/10-roadmap.md` 的对应任务状态、依赖和实际验证记录。
3. 更新相关需求/架构/接口/存储/安全文档；关键决策变更记录到 `docs/12-decisions.md`，注明日期、理由与影响。
4. 增删文档时同步本文和 `docs/README.md` 的链接；修改契约时同步 Rust/TS 类型及相关验证方案。
5. 报告改了什么、静态检查了什么、哪些编译/运行验证因用户约束未执行。不要为了满足验证清单而擅自执行命令。

## 7. 实现基本规则

- Vue 文件统一使用仓库 Prettier 配置，`htmlWhitespaceSensitivity: "ignore"` 仅对 `*.vue` 生效。编写模板时保留清晰的空格、缩进和换行，多行元素的正文、完整闭合标签和相邻组件分别换行，避免 `文字</AppButton` 换行后接 `><AppButton` 的紧贴写法；短元素允许格式化为单行。修改后执行 `pnpm format:vue` 自动整理、`pnpm check:format:vue` 静态校验，均不构建或运行项目。此规则不承诺保留行内元素边界的空白语义；必须精确保留的文本空格使用显式文本表达式，必要时对单个节点使用 `<!-- prettier-ignore -->` 并说明原因，保留 `pre` 等预格式化内容。
- **写代码时使用中文注释。** 新增或修改的行内注释、块注释和文档注释均使用中文；标识符、API 名称和必要的技术术语保持原文。注释说明意图、边界与约束，避免重复代码含义。
- 前端 pnpm；沿用 Vue 3 Composition API、严格 TypeScript。Rust 当前 edition 2024。暂不切 React，也不引入 WebSocket/HTTP 后端。
- 禁止默认接受所有 SSH 主机密钥、明文保存密码/私钥、默认开启弱算法、自动执行远端输出、自动重放断线前输入。
- 自定义命令必须纳入 `AppManifest::commands` 对应 ACL 并校验调用窗口及资源所有权；只写 capability JSON 不足以保护默认自定义命令。
- 不在全局锁内进行网络/数据库/Channel 等待；不能让输出 ACK 堵塞会话关闭、KeepAlive 或信任响应。
- 后续依赖落地时重新核对具体 release 的 API、features、MSRV、许可证、安全公告，更新锁文件和预研；`latest` 页面不是版本锁定策略。

## 8. 交互实现约束

- 主机页左侧使用 Naive UI 文件夹树，`(n)` 为含后代的主机数，不随搜索变化；右侧默认只显示选中组直属主机，可勾选包含子组，收藏主机置顶，不设全部/收藏切换。包含子组及展开组UUID持久化到app.hostsView，旧设置默认false/空数组；即时偏好串行保存且不被设置旧草稿覆盖。顶部独立NSelect搜索全部主机（无有效关键词不显示下拉，focusInput获取真实输入焦点，鼠标勾选字段保留输入光标），结果不改变下方组筛选；支持静态拼音表的全拼/首字母/多音字/混合匹配，配置来源见resources。全局新建标签页弹窗复用useHostSearch和SearchPreferences，结果直接列出、空查询显示收藏优先的保存主机，最多100条；快速连接入口位于弹窗footer，退场后再打开快速连接表单。标签栏＋、Cmd/Ctrl+T及原新连接动作均打开该弹窗；旧keymap补新增动作，冲突或已达绑定上限不抢占已有配置。搜索设置位于设置页：拼音默认开启，name固定勾选，address/username/tags/group默认全选；下拉顶部字段复选框共用app.hostSearch并即时保存，关闭拼音保留原文不区分大小写匹配。新建分组使用库有边框按钮。Pointer Events 支持主机入组/移至未分组、分组修改父级/回到顶层，超过拖动阈值显示源DOM的65%透明快照并淡化原位置，预览穿透指针且提示落点，取消/失焦/切页/捕获丢失清理；前后端均防环；编辑/移动菜单提供键盘替代。删除默认保留直属主机和子组并上移；选择连同内容删除时再次弹窗确认整棵子树，后端事务清理主机与专属凭据，共享凭据和已建立会话保留。
- WorkspaceStore 分离终端 tabs 与标签 groups；activeId 指组，active 指聚焦终端。布局树只计算矩形，TerminalStage 扁平 keyed 宿主列表保证排序、合并、缩放不重建 xterm。窗格可拖回标签或跨组重分割，取消恢复活动标签；详见 [工作区设计](docs/18-terminal-workspaces.md)。
- 广播仅驻内存、默认关闭；成员重组自动关闭，排序和比例调整不关闭。inputOrigin.ts 隔离固定 xterm 6.0.0 内部 onUserInput 标记，配合公开 onData/onBinary 区分用户输入与自动应答，能力缺失禁止开启广播。inputRouter.ts 只转发用户输入和远端滚轮，排除点击、拖动、设备应答及认证；粘贴确认一次，各目标独立编码和代次检查，禁止自动重放。
- SFTP 绑定单 SSH 终端；Workspace 与本地终端隐藏 SFTP 开关，开启侧栏时 canSplit 同时阻止作为来源和目标，排序仍允许。隐藏保留任务，断开/关闭终端取消，重连不恢复旧任务。
- SFTP 每页最多100项、视图10000项、READDIR包256 KiB；列为选择框/名称/大小/权限。目录及目录链接优先，大小按1024进制两位小数，根目录禁用上级。“显示隐藏文件”由全局sftpShowHidden控制，自动保存至数据库app设置，终端侧栏和双栏同步。READLINK/STAT识别链接，保留逻辑路径，由服务器跟随目录链接。
- 上传直接写最终文件，同名普通文件明确确认后覆盖，不依赖远端hardlink；已有目录只合并，不移除目录处理冲突。取消可能留下部分目标，不承诺回滚。下载以本地同目录独占临时项及hard_link无覆盖发布，文件正文仅Rust流动；64 KiB块、全局4任务、每浏览器1操作。
- 外部拖入侧栏仅上传至当前目录，Rust签发owner-bound一次性dropId，不接受前端任意本地路径。拖出令牌由命令统一持有并显式传入原生层，收尾完成前不允许重叠手势；双栏另用Pointer Events进行栏间对拷，不调用原生拖出；内部拖出期间屏蔽上传回流；软件内Drop直接取消拖出，上传须先有带真实路径的外部Enter，空路径回流忽略。macOS使用文件/目录承诺，外部请求兑现后才显示下载；取消或应用内松开不留传输记录。Windows/Linux原生拖出未实现。
- 完整 SFTP 契约、资源限额及待验收范围见 [SFTP设计](docs/16-sftp-workspace.md)，不要在协作指南追加逐轮修复日志；文档保留最终状态和有效决策。

本地终端启动提示为“启动本地终端”，重开提示为“重新打开本地终端，保留终端历史”；SSH 保留原有连接提示。

本地终端基础源码已落地：系统 Shell（Unix 为 SHELL 或 /bin/sh，Windows 为 COMSPEC 或 cmd.exe）、用户主目录、继承环境、PTY/resize/退出码、关闭与原缓冲重开，复用核心 actor/Channel/ACK/窗口所有权。无 SFTP 侧栏。自定义 Shell/cwd/env 设置及完整跨平台进程树清理仍待补齐，三平台编译运行未验收。

独立设置窗口由 `src/modules/settings/SettingsWindow.vue` 装配，`useLiveSettings` 负责校验与自动应用，`useSettingsSync` 负责跨窗口同步；Rust窗口创建位于 `src-tauri/src/settings_window.rs`，设置命令独立于通用数据命令。`settings` capability 仅开放设置和JSON导出，通用会话owner仍仅接受main。设置改为单实例独立窗口（macOS应用菜单、Windows/Linux标题栏应用菜单，settings.open复用可配置快捷键），所有有效修改自动持久化并跨窗口热应用；设置专用Rust队列合并局部字段，保存失败保留输入并提示重试。仅静态验证，三平台窗口/菜单/并发与失败恢复待运行验收。

保险箱由VaultLayout装配，侧栏为主机、凭据、主机信任、连接历史；主机信任独立位于hosts/KnownHostsPage，连接历史独立位于history/ConnectionHistoryPage。两页各自按需加载、独立滚动，支持刷新与失败提示；撤销信任及清空历史保留确认。设置窗口移除原“信任与历史”。固定标签内部ID仍为home，返回保险箱仍默认主机并聚焦搜索。源码已实现，仅静态验证，导航/确认/权限拒绝待运行验收。

固定标签“保险箱”和“SFTP”分别在文字前显示16px保险箱与文件夹线框图标，通过NIcon承载并跟随标签文字颜色；顶部/左侧布局共用，装饰图标不参与无障碍名称。仅静态检查，实际视觉待运行复验。

保险箱侧栏四项导航通过NMenu的icon渲染入口显示18px线框图标：主机用终端屏幕、凭据用钥匙、主机信任用勾选盾牌、连接历史用历史时钟；图标由layout/VaultNavigationIcon集中维护，跟随库菜单文字颜色且不参与无障碍名称。仅静态检查，视觉待运行验收。

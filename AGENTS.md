# Termvexa 开发协作指南

最后更新：2026-10-15。适用于整个仓库。文档语言默认中文。

## 1. 项目定位与范围

Termvexa 是基于 Tauri 2 + Rust + Vue 3 + TypeScript 的桌面终端客户端，长期以 Termius 的桌面核心体验为功能参照。首期只实现 SSH，架构保留 Local Terminal、Serial、SFTP、隧道扩展。仅支持 Windows、macOS、Linux；Android、iOS 不在计划内。对标功能与体验，不复制品牌、素材或专有实现。

## 2. 重要执行约束

- **不要主动构建、运行或启动项目。** 未得到用户明确要求时，不执行 `pnpm dev`、`pnpm build`、`pnpm preview`、`pnpm tauri dev/build`、`cargo build/run/check/test/clippy`，也不通过 IDE、脚本、测试、CI 或其他命令间接触发编译、启动、打包。`cargo check`、Rust 测试同样会编译，不能作为绕过方式。
- **允许按当前任务需要主动安装/更新依赖库、下载必要工具、更新锁文件，以及执行不构建/运行项目的静态检查，无需为这些操作单独确认。** 安装若会自动触发项目构建或运行，先禁用相关生命周期脚本，不能借依赖安装绕过上一条限制。
- 不主动启动 SSH 测试服务器、连接真实主机或串口；这些运行验证在用户明确要求的范围内执行。
- 可进行源文件阅读、联网查阅官方资料、差异检查、Markdown 链接检查等不会构建/运行项目的静态检查。报告中明确区分“静态检查”与“编译/运行验证”；未验证就写未验证。
- 仅编写文档时不为示例提前创建空 crate、添加无关依赖或改写模板业务代码；开发任务所需的依赖安装按上面的允许规则执行。
- 用户已有修改不得覆盖；不要自行提交、推送、发布或创建真实凭据。

## 3. 当前真实状态

| 项目 | 状态 |
| --- | --- |
| 总体阶段 | M1/M2 SSH 首期主体源码已落地，处于体验修复与集成验收阶段；不能标为 MVP 已验收 |
| 主机与首页 | 主机/分组/收藏/标签/搜索、快速连接；首页主机/凭据/内嵌设置，切页保活与统一页头；待完整运行验收 |
| 工作区标签 | 自定义 WorkspaceTabs，固定首页/SFTP、顶部/左侧布局、(n)重复编号、OSC 0/2远端标题后缀（仅展示、不落盘）、关闭确认、右键菜单/键盘导航（配色子菜单受视口限高并使用库滚动条）；主题状态色/圆角、方形新建与会话区滚动已落源码；动态标签拖拽排序、四向拖入合并/嵌套分屏、窗格拖回Tab/跨组重分割、标签后方整区末尾插入（末项后方与空白共用唯一提示线）与比例调整已落源码；待运行验证 |
| SSH 与认证 | core/ssh/storage/security + 桌面命令；密码/公钥、动态 OTP、主机密钥确认/变更阻断、PTY/resize/KeepAlive；密码/私钥/多轮认证互通矩阵待验收 |
| 终端生命周期 | TerminalRegistry 按 terminalId 保活；本地状态消息独立模块，已移除无条件恢复旧光标的退出序列（登录/重连覆盖修复待复验）；断线/失败保留、原终端重连、二进制 Channel/ACK/有界输入输出；全屏/压力/三平台待验收 |
| 工作区广播 | 分屏标题关闭按钮右侧增加广播开关；当前组共享状态，激活实线/其他虚线，标题状态点与文字使用flex居中，标题使用主题正文亮色；状态线使用窗格内部覆盖层避开底部裁剪边缘（残线修复待复验）；键盘/IME/粘贴与远端滚轮，排除点击/拖动/自动应答；成员重组自动关闭，待运行验证 |
| 输入与鼠标修复 | 根页面滚动隔离、背景同步、纯修饰键不跳底、macOS Option跳词、链接 mouseup 清理均已落源码；最新修复待用户复验 |
| 字体与配色 | 本地12种字体/实际字体下拉预览，默认 DejaVu Sans Mono；64套提取配色、默认 Pro、自然展开网格/折叠自定义/JSON/热应用；关闭粗体提亮；选区蒙色独立适配模块待视觉复验 |
| 通用 UI 与设置 | Naive UI 2.45.3（工作区标签除外）；弹窗 footer、设置底部固定保存、默认禁止无关文字选中、快捷键分类/录制/多绑定/导入导出；待 WebView 验收 |
| 存储与凭据 | schema 1 + 独立新库；公共密码/私钥（可选解密密码）、主机凭据引用/专属密码；OS keyring，无明文回退，不兼容旧库；系统服务与故障恢复待验收 |
| 安全与发布 | AppManifest/ACL、main owner、CSP、外链白名单已落源码；安全运行复核/依赖风险处置/三平台发布未完成 |
| 后续范围 | SFTP 仅固定双栏未连接布局；文件操作/传输、隧道/跳板/Agent、Local/Serial、布局持久化/恢复未实现 |
| 验证证据 | 用户有 macOS 编译/终端使用与截图反馈；不能外推为当前版本完整验收。代理仅静态检查，最新修复无运行复验结果；长按回车空白由用户确认是 zsh 行为 |

详细源码入口、验证边界与下一步优先级见 [实施状态当前总览](docs/17-implementation-status.md#0-当前功能总览2026-10-15)。

已移除 greet、移动端入口宏和宽泛默认 opener 权限；原生私钥选择只返回引用，凭据接口不返回秘密。根 Cargo workspace 统一四个领域 crate 与 Tauri 壳，锁文件在仓库根目录。代理没有启动应用、连接真实主机或执行 Rust 编译/测试；用户已反馈实际终端体验，三平台验证未完成。

凭据与主机模型按用户要求重构且不考虑旧数据库兼容迁移：主机直接密码非空自动保存为专属 OS item，显式空值移除并改为连接时询问，无记住开关；元数据更新省略密码则保留。私钥正文与可选解密密码整体导入凭据库，不再独立保存口令凭据，连接不依赖原文件。新基线独立使用 `termvexa-credentials-v1.sqlite3`，旧 `termvexa.sqlite3` 与系统凭据不修改或删除，也不迁移；用户已确认旧路径因版本 2 高于新 schema 1 导致启动失败，路径隔离修复待复验。

## 4. 架构约定

当前已建立 `src/modules/`、`src/services/desktop/`、`src-tauri/src/commands/` 及 core/ssh/storage/security 四个 crate。完整目标结构见 [架构文档](docs/03-architecture.md)，后续领域仍是计划。

- `src/modules/*`：hosts、credentials、terminal、workspace、settings，以及后续 files/transfers；Vue/Pinia 只保存轻量状态，不保存终端输出或文件正文字节。CommandRegistry 统一动作与快捷键；终端配色优先级见 terminal/registry.ts，转换见 settings/themes.ts，选区合成薄适配见 terminal/theme.ts。KeymapService/TerminalThemeService 为设计职责名，不能假定存在同名类。
- `src/services/desktop/`：唯一 Tauri IPC 门面；组件不直接调用 `invoke`。
- `src-tauri/src/`：命令 DTO、窗口所有权校验、Channel 桥接、依赖组装；不承载协议核心逻辑。
- `crates/terminal-core/`：不依赖 Tauri/Vue/russh 的会话模型、能力、actor 控制、生命周期和扩展协议。
- `crates/terminal-ssh/`：russh、主机密钥校验、认证、PTY/Shell、KeepAlive；依赖 core，通过接口使用凭据与信任仓库。
- `crates/terminal-storage/`、`crates/terminal-security/`：SQLite 与操作系统凭据存储适配器。核心定义端口，壳负责注入。
- Local/Serial 的工厂与配置扩展点先保留；具体 crate 在开发对应功能时再创建。SFTP/隧道通过 SSH 连接服务扩展，不塞进通用终端接口。M3 的 `terminal-files` 定义文件服务/传输/冲突端口，`terminal-sftp` 和本地文件适配器实现；本地浏览不依赖 Local PTY，独立 SFTP 不启动 Shell。

会话采用**每会话 actor + 有界字节队列 + 独立取消信号**；重连创建新的后端 session ID，但保留同一 Tab、terminalId 与 xterm 实例，在原缓冲中追加输出。断线、连接失败和后端终态均不得自动关闭 Tab、清空或销毁终端；历史继续可滚动、选择、复制和搜索。TerminalRegistry 按 terminalId 持有实例，不能以 sessionId 变化触发重建。输出走二进制 Channel，控制事件走类型化 Channel，使用消费 ACK 实现端到端背压。所有凭据、信任与网络操作由 Rust 管理。

首页和 SFTP 为不可关闭的固定 Tab，SSH 标签随后；设置 `tabPosition` 控制顶部/左侧布局，旧设置默认顶部。首页内侧栏为主机、凭据、设置。固定页面不创建 session/xterm，切换页面与布局不卸载终端。SFTP 本期仅未连接双栏布局，无文件服务或传输。认证按服务器协商自动补充 Keyboard-interactive，不由用户配置追加 OTP。

前端统一采用 **Naive UI 2.45.3**，已放弃并移除 Reka UI。工作区顶部/侧栏标签按用户要求使用自定义 WorkspaceTabs，支持拖拽排序/合并，标签组与稳定 terminalId 分离；其余所有有库对应能力的交互控件优先直接使用 Naive UI；项目封装只做薄适配。使用 NConfigProvider/库主题与现成样式，不另造控件皮肤，仅保留业务布局、尺寸和终端必要 CSS。终端宿主与工作区生命周期由应用掌控，Tab 显隐不得销毁 xterm。系统文件授权与 xterm 不由 UI 库替代。三平台集成尚未验证，详见 UI 选型文档。

首页主机页保活显隐必须挂在原生 DOM 容器，不能直接给以观察器为根的滚动组件施加 v-show；主机/凭据页各用独立 NScrollbar，设置占满可用高度；三页统一 NH1/NText 页头与页面间距，不显示 eyebrow 或首页品牌/页脚装饰。配色选项以 NGrid 自适应平铺，网格自然展开，不限高且不设独立滚动，随设置正文统一滚动；搜索/导入位于上方，自定义内容用 NCollapse 默认折叠在网格下方，保存为自定义位于编辑区末尾。点击卡片选择全局配色草稿，最终由底部保存设置持久化；展开自定义后才初始化预览。内嵌 AppDialog 将 NScrollbar 正文和底部 footer 分开，设置操作区始终可见。弹窗操作使用 AppDialog 的 `#footer` 并转发到 NModal card footer，正文由库的 content-scrollable 管理，按钮不能放进滚动正文。界面装饰文字默认不可选，输入/编辑区、终端自身选区和显式可复制正文保留选择能力。

快捷键设置分应用内全局与终端类别（含修饰键点击链接）；系统后台热键另行 opt-in。Cmd+L 的 Local 动作待 M4 工厂可用才启用。终端字体固定为12个随应用分发的家族，默认 DejaVu Sans Mono；字体、来源清单和许可证位于 `public/fonts/`，CSS 别名与异步字宽重算避免系统字体版本差异；旧自由字体串读取时归一为默认项，保存由 Rust 白名单校验。终端配色独立于应用主题，内置64套从用户指定解包数据静态提取的方案，默认 Pro；前后端共用 `resources/terminal-palettes.json`，提取脚本与来源摘要随仓库保留；旧自有预设已移除，旧明暗引用读取时分别归一至 Nord Light/Dark，同名 Ocean 使用提取值，用户自定义方案保留。配色支持任意取色与 JSON 导入，热切换不清历史。SFTP 左/右/底部可隐藏面板与双栏 Tab 共用后端，远端互传默认有界内存中转，上传/下载/互传均检查覆盖并确认；原生拖出必须单独验证，不能用 Web DnD 替代。

## 5. 文档索引与阅读顺序

| 文档 | 用途 |
| --- | --- |
| [docs/README.md](docs/README.md) | 基线、术语、文档导航 |
| [01-product-requirements.md](docs/01-product-requirements.md) | 范围、需求编号、MVP 验收 |
| [02-library-research.md](docs/02-library-research.md) | 联网预研、来源、版本观察与待验证项 |
| [03-architecture.md](docs/03-architecture.md) | 分层、依赖、目标目录、扩展方式 |
| [04-session-model.md](docs/04-session-model.md) | actor、状态机、trait、清理语义 |
| [05-ssh-design.md](docs/05-ssh-design.md) | SSH 握手、信任、认证、PTY、异常 |
| [06-ipc-and-streaming.md](docs/06-ipc-and-streaming.md) | 命令、DTO、二进制帧、顺序和背压 |
| [07-storage.md](docs/07-storage.md) | 表结构、迁移、凭据引用与一致性 |
| [08-security.md](docs/08-security.md) | 威胁模型、ACL、CSP、安全默认值 |
| [09-desktop-ux.md](docs/09-desktop-ux.md) | 主机管理、标签、终端生命周期、桌面交互 |
| [10-roadmap.md](docs/10-roadmap.md) | 里程碑、任务依赖、风险、进度 |
| [11-validation.md](docs/11-validation.md) | 后续验证方案、测试矩阵、性能目标 |
| [12-decisions.md](docs/12-decisions.md) | 架构决策与变更条件 |
| [13-ui-stack.md](docs/13-ui-stack.md) | Naive UI 固定版本、控件映射、主题与桌面集成边界 |
| [14-shortcuts.md](docs/14-shortcuts.md) | 全局/终端快捷键、修饰键点击、冲突/焦点/配置与原生边界 |
| [15-terminal-themes.md](docs/15-terminal-themes.md) | 内置/自定义配色、取色编辑、导入导出与热应用 |
| [17-implementation-status.md](docs/17-implementation-status.md) | 首期源码交付、静态验证证据、已知限制与运行验收入口 |
| [18-terminal-workspaces.md](docs/18-terminal-workspaces.md) | 标签排序、嵌套分屏、输入广播、焦点与保活边界 |
| [16-sftp-workspace.md](docs/16-sftp-workspace.md) | SFTP 可停靠面板、独立双栏、拖入拖出、属性与安全传输 |

开发前至少阅读本文、目标模块文档及路线。接口冲突应修订文档与实现，不临时增加第二套契约。

## 6. 修改后的文档同步义务

每次实现功能、修改架构、接口、数据结构、安全策略、依赖选型或开发范围，**在同一轮修改中**：

1. 更新本文件的“当前真实状态”、架构说明与必要索引；不要把计划标成完成。
2. 更新 `docs/10-roadmap.md` 的对应任务状态、依赖和实际验证记录。
3. 更新相关需求/架构/接口/存储/安全文档；关键决策变更记录到 `docs/12-decisions.md`，注明日期、理由与影响。
4. 增删文档时同步本文和 `docs/README.md` 的链接；修改契约时同步 Rust/TS 类型及相关验证方案。
5. 报告改了什么、静态检查了什么、哪些编译/运行验证因用户约束未执行。不要为了满足验证清单而擅自执行命令。

## 7. 实现基本规则

- **写代码时使用中文注释。** 新增或修改的行内注释、块注释和文档注释均使用中文；标识符、API 名称和必要的技术术语保持原文。注释说明意图、边界与约束，避免重复代码含义。
- 前端 pnpm；沿用 Vue 3 Composition API、严格 TypeScript。Rust 当前 edition 2024。暂不切 React，也不引入 WebSocket/HTTP 后端。
- 禁止默认接受所有 SSH 主机密钥、明文保存密码/私钥、默认开启弱算法、自动执行远端输出、自动重放断线前输入。
- 自定义命令必须纳入 `AppManifest::commands` 对应 ACL 并校验调用窗口及资源所有权；只写 capability JSON 不足以保护默认自定义命令。
- 不在全局锁内进行网络/数据库/Channel 等待；不能让输出 ACK 堵塞会话关闭、KeepAlive 或信任响应。
- 后续依赖落地时重新核对具体 release 的 API、features、MSRV、许可证、安全公告，更新锁文件和预研；`latest` 页面不是版本锁定策略。

终端分屏按用户要求提前至 M2：WorkspaceStore 分离终端元数据 tabs 与标签组 groups，activeId 指组、active 指聚焦终端。二叉树只计算矩形，TerminalStage 扁平 keyed 宿主列表保证排序/合并/缩放不移动或重建 xterm。非激活单终端标签可四向拖入当前窗格，形成 Workspace 并嵌套分割；组可排序，分隔线可调整，关闭窗格收缩树。固定页不参与，窗格标题可拖回标签栏为独立Tab或在同组/其他组重新分割，跨组拖动可悬停目标标签600ms切换；取消拖动恢复原标签。整体 Workspace 合并/布局保存恢复未实现；详见18文档，运行验证未执行。

工作区广播仅驻内存，默认关闭；组内窗格增减/重组自动关闭，排序/比例调整不关闭。inputOrigin.ts 隔离固定xterm 6.0.0内部onUserInput事件订阅（不修改库），配合公开onData/onBinary区别用户输入与自动应答，缺少标记能力禁止开启广播；升级必须复验。inputRouter.ts 只分发聚焦终端用户输入与远端滚轮，不转发点击/释放/拖动/设备应答/认证弹窗；粘贴确认一次并按目标独立编码，复用各会话FIFO和代次，不记录输入、不自动重放。

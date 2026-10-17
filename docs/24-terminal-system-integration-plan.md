# 系统终端集成与启动本地标签实现方案

设计日期：2026-10-17。**全文为待实现方案，不是当前运行行为。** 平台事实及官方来源见 [功能调研](23-terminal-system-integration-research.md)。本轮只交付文档，不新增代码、依赖、迁移或系统注册项。

## 1. 目标与交付边界

第一目标是让用户设置“启动进入本地终端”，普通启动在完成保险库解锁后自动创建一个 Local TAB。第二目标是从命令行及目录菜单打开 Termeta，并在已有实例内创建正确目录的新 TAB。后续分别实现平台默认关联及 Linux 默认启动器，Windows 系统控制台委派独立研究。

不将“启动一个新 Shell”“接收文件/URL”“接纳 Windows 已存在的 console client”混为同一种动作。每条入口通过 Rust 统一规范化后进入对应会话流程；输出、凭据与原生句柄不进入启动 DTO 或 Pinia。

## 2. 阶段 A：普通启动自动打开 Local TAB

### 2.1 设置与用户行为

新增可选设置字段 `startupPage: "vault" | "local"`，旧记录缺失时补 `"vault"`；新安装默认也为 `"vault"`，用户可在常规设置切换到“本地终端”。设置属于本机加密 `settings.app`，不进入云备份，也不等于登录系统自动启动应用。

阶段 A 沿用 `schemaVersion: 1` 和 setting `value_version = 1`，作为向后补齐的可选字段；Rust 白名单、缺失默认、非法值拒绝及 TS 默认必须同步。无 SQL 结构变化，不修改任何已应用迁移。若未来改变设置语义再制定版本升级，不顺手复用数据库 schema 号。

修改设置即时保存，**只影响下一次普通冷启动**；收到跨窗口 `settings-changed` 不立即新建 TAB。固定保险箱仍存在，用户可随时切回。

| 触发 | 预期行为 |
| --- | --- |
| 普通冷启动，startupPage=local，OS 自动解锁成功 | 设置加载且主窗口终端宿主就绪后，创建并激活一个 Local TAB |
| 普通冷启动，需要首次建库或密码解锁 | 先显示 VaultGate，成功后应用启动偏好；失败/取消不启动 Shell |
| 普通冷启动，startupPage=vault | 保持保险箱 |
| 启动过程已经收到明确外部打开请求 | 优先处理明确请求，不额外创建偏好 TAB；非法明确请求显示错误，不隐式回退新 Shell |
| 再次点击图标、single-instance 无参数激活、macOS Reopen | 显示/恢复聚焦；不重跑冷启动偏好 |
| 手动点击新建本地终端 | 按现有命令创建，不受“启动偏好已消费”影响 |
| 设置窗口、云恢复通知、重连、睡眠恢复 | 不触发启动偏好 |
| Shell 创建失败或正常退出 | 保留 TAB 和历史，手动重开；不循环自动创建 |

### 2.2 一次性协调与就绪条件

在主窗口引入启动协调器，维护当前进程的启动批次与 `waiting → applying → consumed` 状态。`consumed` 在分配 terminalId 并登记创建动作时设置，不依赖 Shell 连接成功；失败也不在设置更新、组件重挂载或解锁回调中重试。

执行条件为 `main`、保险库已解锁、设置已读取、WorkspaceStore/TerminalStage 可承载、初始启动意图已收集。继续通过已有 `workspace.add` 与 TerminalRegistry/open_session 创建，尺寸、Channel、ACK、历史及失败保留均沿用当前链路。

当前 `loadUnlocked()` 使用 `Promise.all` 同时等待主机、转发和设置；实施时拆分“保险库/设置/终端宿主就绪”与各业务列表加载结果。Local 不应因主机列表加载失败而永久无法打开；主机/转发错误保留各自可见提示，不能把解锁失败当作可忽略错误。

阶段 A 的最小改动可只使用内存启动协调，不要求先实现下一阶段的外部 IPC 队列。后续扩展时让两者共享启动决策，避免保留第二套自动打开逻辑。

## 3. 阶段 B：CLI、cwd 与统一启动请求

### 3.1 CLI 契约

以下为计划接口，不是当前可执行命令。本阶段只开放交互 Shell 和目录，不提前承诺任意命令执行：

```text
termeta                              普通激活；已运行时只聚焦
termeta --local                      明确新建一个默认本地终端
termeta --local --working-directory <绝对或相对目录>
termeta --local --cwd <目录>           上一参数的别名
termeta --help
termeta --version
```

明确 `--local` 请求即使已有 Local TAB，也创建新的 TAB；无参数二次调用只激活。每个新调用具有新请求 ID，同一请求的重投不应多开。

CLI 启动时立即捕获调用进程 cwd，相对 `--working-directory` 基于该 cwd 解析。没有显式目录时：明确 CLI Local 使用调用者 cwd；普通 GUI 的启动偏好和应用内手动新建使用现有主目录默认。不能调用 `set_current_dir` 改变主应用全局 cwd。

初始解析使用 `args_os` 和原生路径；单实例 2.4.2 当前仅转发字符串，支持范围先明确为可无损传输的路径。发现不支持的非 UTF-8 路径时应在进入有损转换前拒绝；如果插件先于解析发生转换，需调整解析/转交顺序或采用原生编码 launcher 通道，不能只修改回调接收端。Windows 空格、Unicode、UNC 和网络目录延迟分别验收，目录元数据检查放阻塞任务并可取消。

Windows 主程序是 GUI subsystem，独立设计 CLI 帮助和错误输出（例如附着调用者控制台的轻量 launcher）。初期不承诺等待子命令结束或返回其退出码；launcher 的成功只代表请求被接收。暖启动投递需返回可解释的接收/拒绝结果，不能把第二实例退出码 0 解释为 Shell 已启动。

### 3.2 Local 类型扩展

统一扩展现有 `OpenTarget::Local` 与 `SessionConfig::Local`，不新建旁路 `open_local_shell`：

```ts
// 计划类型；注释描述阶段 B 的兼容边界。
interface LocalConfig {
  configVersion: 1;
  cwd?: string; // 缺省沿用主目录；进入 Rust 后再次校验。
}
type LocalTarget = { source: "local"; config?: LocalConfig };
```

兼容现有 `{ source: "local" }`，Rust/TS 同轮更新并拒绝未知字段。core 的 Rust 配置使用适合原生路径的类型；平台层完成输入转换，不在协议核心加入 LaunchServices、注册表或 Tauri 依赖。

`LocalFactory` 在创建新进程前验证 cwd 存在且为目录，并通过 `CommandBuilder::cwd` 设置。明确提供但不存在/无权限的目录应失败保留 TAB，不能静默退回 HOME。重开沿用该 TAB 的 Local 配置快照；普通新建/复制/分屏明确传播配置，保持 terminalId 跨重连稳定。

此阶段不开放 env 或自定义 Shell 路径；后续作为 EXT-01 子任务独立实现，避免目录入口扩大执行能力。

### 3.3 Rust 统一请求队列

建议在 `src-tauri/src/launch/` 放置平台无关解析、队列及平台适配，名称表示拟新增模块。现有 lib.rs 只编排注册与事件；core 不依赖这些模块。

```text
argv / single-instance / macOS Opened / Services / URI
                         ↓
           Rust 解析、来源标注、限额、规范化
                         ↓
         有界 LaunchQueue（先建队列，再开放入口）
                         ↓
           main 解锁 + 设置/终端宿主 ready
                         ↓
       领取请求 → workspace.add → terminalId 登记
                         ↓
           现有 open_session + Channel / ACK
```

拟定内部模型包含 `requestId`、`receivedAt`、`source`、`action`；阶段 B 的 action 仅 `Activate` 与 `OpenLocal { cwd }`，后续再加入结构化远端/脚本/委派类型。`source` 用于策略和诊断，不是鉴权凭据；本地进程也可伪造 CLI/URL。

队列建议最多32项、每项规范化载荷上限64 KiB、待处理 TTL 5分钟。超限拒绝并提示；锁定时只暂存内存，显示待打开数量及取消入口，不创建进程，不写日志或数据库。到期后丢弃并提示，可由用户重新发起；取消解锁/退出清理请求。不得在稍后重启时重放脚本或命令。

不能只向 WebView 发一次 event：前端未监听会丢失。采用“Rust 保留队列 + 通知仅作唤醒 + main 主动拉取”的模型，拟新增 IPC 如下；这些名称尚未纳入当前命令清单：

| 拟新增命令 | 窗口及门禁 | 结果 |
| --- | --- | --- |
| `claim_launch_requests` | 解锁后的本地 main；有界批量 | 领取规范化轻量请求及 claim token |
| `complete_launch_request` | 同一 main / token | 登记 terminalId 或拒绝原因，幂等结束 |
| `discard_launch_requests` | 本地 main；锁定时仅取消自己的待处理队列 | 丢弃指定请求，不执行动作、不返回敏感正文 |

新增命令必须同步 `command_names.rs`、AppManifest、自定义权限、capability、Rust handler 和 `src/services/desktop` 门面。settings 无领取/创建会话权限。锁定取消使用窄化校验，不放宽通用 owner，也不给锁定窗口读完整启动载荷。

请求领取后先登记 `requestId → terminalId`，再创建视图并沿用稳定 open_session requestId；同进程补领按映射恢复/返回结果，不能新建第二个 TAB。claim 超时不意味着可以重新执行任意命令；进程或 WebView 崩溃后不得自动重放执行类请求。此方案的幂等保证限于当前应用生命周期，不能宣称跨崩溃 exactly-once。

冷启动 argv 与后续系统事件都汇入队列。相同传输事件的请求 ID 用于去重；相同 cwd/URL 的两次独立用户动作仍是两次请求，不能仅按载荷哈希长期吞掉。macOS 首次 URL 事件到达时机要在安装包原型验证；即使事件迟到，也必须执行明确请求且不丢失，验收“无额外偏好 TAB”后再开放该入口。

每类系统事件只设一个采集者：CLI launcher 生成可跨转交重投保留的请求 ID；不同时让手工 argv 解析和 deep-link 插件对同一 URI 入队；macOS `Opened` 与插件回调二选一。若未来采用插件的初始查询加后续订阅，须在 Rust 收件层形成同一事件记录，不能让 JS 各生成一个 ID 后再期待去重；没有稳定事件标识时优先消除重复采集路径。

## 4. 阶段 C：目录菜单与终端身份

| 平台 | 实现位置与行为 | 安装/撤销边界 |
| --- | --- | --- |
| macOS | Info.plist `NSServices`，AppKit Services provider；pasteboard 目录 URL → OpenLocal；同时处理 Tauri Opened/Reopen 的不同语义 | 自有“在 Termeta 中打开终端”服务；安装、服务刷新、更新、卸载实测；不替换 Apple 自带服务 |
| Linux | 按实际包产物生成 `.desktop`，`Terminal=false`、`System;TerminalEmulator;`；提供 CLI PATH 入口 | deb/rpm/AppImage 分别验收；只有声明所需 CLI 完整后才加执行元数据和默认选择适配 |
| Windows | 每用户 Explorer Directory 与 Background 静态 verb → 带 cwd 的 CLI；系统传入路径作为独立参数 | 保留正确 exe/参数引用，卸载只删自己拥有的项；现代菜单另需身份和 IExplorerCommand |

macOS Services 在 AppKit 主线程注册/取 pasteboard，通过 `NSSendFileTypes` / `NSRequiredContext` 声明输入和上下文；服务端仍需验证为可支持的本地目录，后续路径检查进入 Rust 队列，不在回调等待解锁或网络。多目录选择不偷偷无限开 TAB：按32项总限额显式提示，取消不会执行。声明依据见[Apple Services Properties](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/SysServices/Articles/properties.html)。

注册操作与“设置默认”分开显示；安装更新可维护自己的必要注册，不强行改用户默认关联。明确提供启用/移除入口，外部写入失败保留现状并显示真实结果。系统集成状态从系统读取，不能仅保存一个 `isDefaultTerminal=true`。

## 5. 阶段 D：命令执行、Linux 默认启动器与关联

### 5.1 结构化命令执行与 Linux 兼容

后续扩展 CLI 为 `-e <program> [args…]` 或 `-- <program> [args…]`，消费分隔符后所有 argv；支持 `-T/--title` 才满足 Debian/相关调用方需求。LocalConfig 增加互斥的 `shell` 与 `exec { program, args }` 启动方式，设置合法默认并明确广播能力。

执行通过进程/PTY 创建 API，保留空参数、引号、Unicode 和以 `-` 开头的参数，不把 argv 重新拼为 `sh -c` / `cmd /c`。需要 Shell 语法时调用者显式传 `sh -c ...` 等；Windows CreateProcess 的命令行编码及 .bat/.cmd 不是 POSIX argv，需单独定义并验证，不提前宣称完全兼容。命令模式退出后保留 TAB，不回退交互 Shell、不自动重启，默认排除广播收发，与现有 SSH 命令模式保持一致。

此阶段完成后才能添加调研文档中的 `X-TerminalArg*` 元数据、适配 `xdg-terminals.list`、Debian alternatives 和 Plasma 默认应用。只支持无参数 Shell 的版本可以注册应用分类，但不能成为声称支持 `-e` 的默认启动器。

默认终端入口必须与普通 GUI 激活区分：计划使用专用 desktop entry，例如 `Exec=termeta --local`，让无命令的冷/暖调用也明确新建 Local。`--local -e ...` 中 exec 覆盖交互 Shell，不创建两个 TAB；Debian alternative 可指向同样语义的 launcher，Plasma 的 desktop ID/可执行后备均需验证无参数行为。不能把仅聚焦窗口的普通 `termeta` 直接当作完全兼容的默认终端入口。

XDG 列表编辑必须保留其他项、注释和桌面专用优先级；撤销只移除本应用项，不覆盖整个文件。Debian 通过包维护脚本注册自身候选及手册，卸载移除；AppImage 不主动写系统 alternatives。GNOME 不写已弃用的默认终端 gsettings，Nautilus 自有菜单独立安排。

### 5.2 URL 与文件默认关联

自有 `termeta:` 可先只支持激活/新建本地交互终端，后续再定义版本化目录或保存项请求。公开 URI **不支持 `command`、任意 env、Shell 路径、脚本执行或原生句柄**；遇到未知字段/动作拒绝，不能借 CLI 参数解析退回任意执行。

`ssh://`、`telnet://` 映射为现有结构化临时连接，严格处理 user/host/port/IPv6/百分号编码。禁止 URL 内密码、远程命令和通过用户信息注入额外选项；仍由 Rust 进行主机信任和认证，Telnet 明文提示保留。入口不向现有终端注入 `ssh ...` 或 `cd ...` 文本，不广播、不保存 OTP 或 URL 秘密。

macOS 使用现代 NSWorkspace 异步默认关联 API，按最低系统版本决定是否兼容旧接口；文件类型与 URL scheme 分别检测/设置。设置成功、用户取消、系统拒绝及后续外部更改分别呈现。Linux MIME handler 和 Windows 协议 handler 也只代表 URL 关联。

`.command`、`.tool` 等文件打开另立脚本请求类型：校验本地路径、文件类型/权限、shebang 和 cwd，用户看到将运行的文件后确认；系统 quarantine/Gatekeeper 不做移除或绕过。原型需要对比原生进程创建与 Finder/LaunchServices 的隔离文件行为，不能把“不删除 quarantine”视为已经保留相同系统安全交互。完成接收、执行、关闭语义及现代 UTType/旧 Shell role 原型后再提供相关“默认打开”入口，不预先接管泛化可执行文件类型。

## 6. 阶段 E：Windows 系统默认终端专项

### 6.1 必须先解决的结构问题

当前 `LocalFactory` 表达“创建进程”，不能接纳已经存在的 console client。拟新增 Windows inbound adapter / 能力注册表，由 Rust 持有句柄；core 只接受无平台细节的会话能力引用，不能让 Vue 提供数字 HANDLE、PID 或可猜测 token。

若采用 broker，它负责 COM class factory、选定版本 Handoff、句柄 RAII、client/server 生命周期及向 UI 分配一次性接管能力；通过同用户且校验完整性级别的 IPC 与主进程协调，跨进程用 DuplicateHandle 等原生能力。普通 CLI 和此通道分开验证，不能把伪造 CLI 参数当作可信委派。

委派时 client 已存在，不能长期堵在“等待解锁”的普通 LaunchQueue，也不能为了避开门禁重新启动 client。产品必须先选定方案并关闭门槛：

1. 继续要求解锁：在 handoff 承诺接管前验证 UI/保险库可用；不能接管时按已验证协议拒绝，让系统处理可行的回退。接管后的失败是否仍能回退需单独验证，不能假设总能退回 conhost。
2. 独立提供锁定前 Local：需新增不访问保险库的窗口/会话权限、历史策略及最小设置，保持主机/凭据/文件/转发锁定。必须另立安全和存储设计，不能把所有 owner 改为免解锁。

阶段 A–D 保留既有门禁，阶段 E 推荐先验证第一条的可行性；未解决锁定与失败接管行为前，不发布为系统默认候选。

管理员 console client 不能无条件交给中等权限已运行实例；需要提权隔离实例/broker、独立会话资源及路由键，不强制全应用管理员运行。当前单实例策略要在解析委派启动及权限分流后再决定转交，不能让插件提前杀掉承载管理员 handoff 的进程。

### 6.2 包装与原型退出条件

固定 Windows 11 基线及可选 Windows 10 22H2+补丁基线，选定一组 console host、Handoff IDL、proxy/stub release。验证同包双 `windows.appExtension`、COM server/interface 声明，比较完整 MSIX 与 external-location identity package 的可行性。选择包身份时保持现有保险库/元数据数据目录稳定，或先设计可恢复迁移，不能因包装变化生成另一套看似丢失数据的目录。

身份包及扩展必须在目标用户下完成实际注册；仅生成/签名 MSIX 或提供商店下载入口不会自动赋予候选发现能力。每用户与全机安装的注册可见性、COM 激活及升级/卸载分别验收。

自有 CLSID/包身份/签名/升级/卸载全程独立。状态分为“未支持 / 未注册 / 已注册可选 / 已被用户选为默认 / 注册异常”，从真实系统信息读取。安装更新不自动写 DelegationConsole/DelegationTerminal；卸载前处理仍指向自身的默认值及可用回退，不能留下失效 CLSID。

委派 session 不新建默认 Shell；初始输出在有界 Rust 通道中承接，UI ready 后接入现有字节流。短命令在 UI 初始化前退出、满队列取消、resize、Ctrl+C、窗口关闭、broker/UI 崩溃、句柄泄漏和进程树回收均是原型门槛。完成真实 POC 再将此设计固化为 Rust/TS 类型与当前接口文档。

## 7. 文件改动清单与工作拆分

下表是后续开发的落点，不要求本轮创建空文件或 crate。

| 阶段 | 主要落点 | 完成判断 |
| --- | --- | --- |
| A | settings model/store、SettingsPage、Rust settings 校验、App 启动协调 | 偏好持久化；解锁后普通启动只开一次；不影响手动新建 |
| B | core LocalConfig、terminal-local、sessions OpenTarget、desktop types/client、WorkspaceStore、拟新增 launch 模块、ACL/command_names | cwd 到进程边界一致；冷暖启动、队列及失败语义完整 |
| C | macOS Info.plist/Services；Linux desktop 模板/安装脚本；Windows 安装器自有 verb | 各自目录入口可安装/撤销；不误报系统默认 |
| D | Local exec/CLI 参数、命令模式能力、Linux 默认适配、scheme/文件处理与状态查询 | 执行边界和真实桌面调用兼容；关联能力逐项验收 |
| E | Windows COM/inbound adapter、可选 broker、包扩展/签名/发行配置 | 真实系统委派、权限、锁定、异常和卸载回退均有证据 |

新增 Local 字段需同步所有模式匹配、标题/重复会话键、复制/分屏/重连快照、历史摘要及错误显示。历史继续只记录“本地终端/local”及安全摘要，不存 argv/env/完整 cwd；工作区存档仍不支持 Local，自动开一个 TAB 不等于退出标签恢复。

## 8. 验证计划与交付标准

### 8.1 可主动进行的静态检查

- 文档相对链接/锚点、差异空白和计划/实现状态一致性。
- 后续源码修改的结构静态检查；修改 Vue 时按仓库要求执行 `pnpm format:vue` 和 `pnpm check:format:vue`。
- Rust/TS DTO、窗口 ACL、命令清单、安装/卸载项及默认状态展示逐项对照；迁移与锁文件差异复核。

### 8.2 需要用户另行明确授权的编译与运行验证

| 范围 | 必测场景 |
| --- | --- |
| 自动 TAB | 全新设置、旧记录、两种偏好、自动解锁/手动解锁/失败重试、列表加载失败、热更新、失败保留与退出后不重建 |
| 外部启动 | 冷启动/暖启动、10次并发独立请求、同请求重投、锁定/取消/过期/超限、WebView ready 前请求、窗口关闭中的迟到请求 |
| cwd/argv | 空格/引号/Unicode/前导短横线/空参数、无权限/失效目录、相对路径、UNC/网络目录、非 UTF-8 明确拒绝、无隐式 Shell 注入 |
| 生命周期 | Shell/命令退出码、短尾输出、Ctrl+C/resize、满队列取消、关闭进程树、重开保留历史且不自动重放命令 |
| macOS | 已安装签名包的 Services/Opened/Reopen、Finder 多目录、NSWorkspace 用户取消、旧 Shell role/现代 UTType、脚本隔离属性 |
| Linux | 默认终端无命令冷/暖调用必须新建 Local；选定发行版 GNOME/Plasma、GLib/KIO/xdg-terminal-exec 版本、deb alternatives、文件管理器自有菜单、AppImage 移动、X11/Wayland |
| Windows 入口 | Explorer 路径引用、Windows 11 现代/传统菜单、安装更新卸载、CLI 可见错误、包身份数据目录 |
| Windows 委派 | Run/Explorer/开始菜单外部 client、冷暖 UI、锁定、提权分流、短命令、注册损坏、broker 崩溃、接管前/后失败与系统回退 |

本轮未执行上表，也未执行项目编译、Rust 测试、启动或系统注册。每阶段完成源码后再更新对应需求，真实验证证据按平台单独记录；不能把调研或静态通过记为系统默认终端已实现。

# 系统终端注册与默认终端调研

调研日期：2026-10-17。本文是功能与平台接口调研，实施契约见 [实现方案](24-terminal-system-integration-plan.md)。本轮只阅读项目源码、官方资料及上游源码，没有实现功能、改变系统默认值、构建或启动项目。上游主分支描述的是调研时行为，不作为依赖锁定版本。

## 1. 结论与能力分层

Termeta 可以提供启动即打开本地终端、命令行打开目录、文件管理器入口以及部分系统默认关联。三平台没有一套含义相同的“注册为 Terminal”接口，产品界面必须按能力说明。

| 能力 | Windows | macOS | Linux |
| --- | --- | --- | --- |
| 普通启动自动打开一个 Local TAB | 应用启动策略 | 应用启动策略 | 应用启动策略 |
| 外部指定目录打开新 TAB | 自有 CLI + 单实例转交 | 自有 CLI + 单实例转交 / Services | 自有 CLI + 单实例转交 |
| 注册终端应用身份/入口 | 安装器、CLI、Shell verb；包扩展另需身份 | `.app`、Info.plist、Services | `.desktop`、`TerminalEmulator` 分类 |
| 文件或 URL 默认打开程序 | 文件/协议关联，独立于控制台宿主 | LaunchServices / NSWorkspace，类似 iTerm2 的部分“默认”能力 | MIME / scheme handler，独立于默认终端选择 |
| 系统默认控制台宿主 | 支持版本上有 console/terminal delegation，需原生接管实现 | 未找到对应的公开通用委派接口 | 没有统一的控制台宿主委派；桌面/发行版分别选择终端启动器 |
| 文件管理器“在此打开” | 自有 Explorer 菜单 | 自有 Finder Services | 按文件管理器及桌面环境集成 |

**建议先实现自动本地 TAB 与统一外部启动入口，再接入各平台。** Windows 的完整默认终端属于独立原型和发布工程；macOS 不承诺接管写死 `Terminal.app` 的调用；Linux 不承诺一次设置覆盖全部桌面及文件管理器。

“替换系统终端”指用户日常入口使用 Termeta，不修改系统 Shell、系统终端二进制或系统保护策略，也不重定向已有 PTY 中正在运行的程序。

## 2. 项目现状与可复用基础

| 已核对源码 | 当前行为 | 对方案的影响 |
| --- | --- | --- |
| [本地驱动](../crates/terminal-local/src/lib.rs) | portable-pty 0.9.0；Unix PTY / Windows ConPTY；Unix 从 `$SHELL` 选择 Shell 并带 `-l`，回退 `/bin/sh`；Windows 从 `COMSPEC` 选择，回退 `cmd.exe` | 已能打开自建本地终端，不等于能接纳 OS 委派的控制台 |
| 同上 | cwd 为 HOME / USERPROFILE，继承应用环境；尚无自定义 Shell/cwd/env/argv | 目录入口需要扩展 Local 配置；Finder 启动环境不保证与交互 Shell 相同 |
| [App](../src/App.vue) | `openLocalTerminal()` 调用 `workspace.add(..., { source: "local" })`；`loadUnlocked()` 等待主机、转发和设置加载 | 普通启动可复用已有入口；应增加一次性启动协调，不能在 mounted 中无条件创建 |
| [会话命令](../src-tauri/src/commands/sessions.rs)、[owner](../src-tauri/src/state.rs) | `open_session` 需解锁；Local 同样先写连接历史，再创建 session | 不能假设 Local 现已支持锁定前使用，也不能只绕过前端 VaultGate |
| [设置](../src/modules/settings/model.ts)、[Rust 校验](../src-tauri/src/settings.rs) | 加密设置，字段白名单，旧值补齐，跨窗口局部保存 | 新启动偏好需 Rust/TS 同步，不需要改 SQL schema |
| [Tauri 装配](../src-tauri/src/lib.rs) | single-instance 2.4.2 已注册，但回调丢弃 argv/cwd，仅显示并聚焦 main | 第二次 CLI 调用当前不会新建带目录的 TAB |
| [Tauri 配置](../src-tauri/tauri.conf.json)、[依赖](../src-tauri/Cargo.toml) | 尚无本功能专用 URL/文件/终端身份声明或 handoff 实现；Windows 发布主程序为 GUI subsystem | GUI subsystem 不等于系统默认终端；CLI 帮助、错误输出还需独立处理 |

现有 Unix 清理只覆盖初始/前台进程组，Windows 完整进程树回收尚未完成；默认终端能力不能先于这些生命周期门槛宣称可用。

## 3. Windows：默认控制台宿主委派

### 3.1 系统支持及工作方式

微软当前文档列出的默认终端支持为 Windows 11 全版本，以及 Windows 10 22H2 安装 2023-05-23 的 KB5026435 或包含该修复的后续累积更新后。它不意味着旧系统也支持，且 Windows 10 的产品支持/维护策略需要另行决定。[W1]

当外部 console client 需要新控制台而没有附着既有控制台时，系统可从 `conhost.exe` 发起 console host 与 terminal host 接管；console API 服务端和显示/交互终端是两种职责。已有控制台、程序显式选择的宿主及特殊创建标记，不能概括为“所有 exe 都强制转交”。[W2]

ConPTY 提供伪控制台创建与 I/O 能力；Termeta 当前通过 portable-pty **自己启动进程**。系统默认终端则要接纳系统交来的 client/console，不能收到命令行后重新执行一次。[W3]

### 3.2 接口、候选发现与注册

| 机制 | 上游可确认的职责 |
| --- | --- |
| `IConsoleHandoff::EstablishHandoff` | console API 服务端接管，涉及 driver、连接消息、事件及进程句柄 |
| `ITerminalHandoff2` / `ITerminalHandoff3::EstablishPtyHandoff` | 终端 UX 接管；Handoff3 由终端创建并返回读写 pipe，接收 signal/reference/server/client 等能力 |
| `IDefaultTerminalMarker` | 空标记接口，不是设置默认值的管理接口 |
| `windows.appExtension` | Name 分别为 `com.microsoft.windows.console.host` 与 `com.microsoft.windows.terminal.host`，Properties 中声明 CLSID |
| `windows.comServer` / `windows.comInterface` | 声明 COM server 及所需跨进程接口代理 |
| `AppExtensionCatalog` | 上游枚举同一 package 的 console 与 terminal 扩展，配成候选项 |

依据为上游 IDL、manifest 和枚举实现。[W3][W4] **不能用不存在的 `Category="windows.terminal"` 配置代替这些机制。** 当前主线的 Handoff3 与早期设计中的句柄方向/签名不同，实施时要固定 OpenConsole、IDL、proxy/stub 版本及支持系统，不把 main 分支当稳定 SDK。

上游将所选 CLSID 写入 `HKCU\Console\%%Startup` 的 `DelegationConsole` 和 `DelegationTerminal`。[W4] 这能解释默认值存放位置，但不是安装器自动改值的授权，也不是通用 Win32 `SetDefaultTerminal()`。先注册可工作的候选，再由用户选择；不要借用 Windows Terminal 私有安装路径或 CLSID。

### 3.3 Tauri 包装与实现成本

Tauri 内置 Windows 安装分发以 MSI/NSIS 为主，不会因 `bundle.targets = "all"` 自动获得上述 MSIX manifest。[W5] 有两条候选路线：

1. 完整 MSIX：微软 2026-10-07 更新的 Tauri + winapp CLI 指南提供 package identity 与包装起点，但没有替应用实现 terminal handoff。[W6]
2. 保留现有安装器，配合 external-location identity package（sparse package）：微软支持为外部 Win32 安装赋予包身份；是否能完整承载本项目所需扩展组合、COM、更新和卸载，要先做原型。[W7]

原生接管可在 Rust 中实现，也可以采用小型 broker 隔离 COM/句柄管理。broker 是工程建议，不是系统硬性要求。还需要可分发的 console host、句柄传递、resize/信号、短命令尾输出、完整性级别隔离和异常关闭策略。OpenConsole/第三方 ConPTY SDK 的版本、许可及分发条件须单独核对，不能假设用户装有 Windows Terminal 即可复用。[W3][W8]

### 3.4 更早可交付的入口

每用户 Explorer 静态 verb 可调用 `termeta --local --working-directory <目录>`；Windows 11 可能出现在“显示更多选项”。现代菜单需 `IExplorerCommand`、包身份及 `windows.fileExplorerContextMenus`，是单独集成。[W9][W10]

CLI、URI、Explorer 菜单均不能让系统将直接启动的 `cmd.exe` 自动交给 Termeta。产品可先提供这些入口，但不能据此显示“已设为系统默认终端”。

## 4. macOS：类似 iTerm2 的默认关联与 Services

### 4.1 “Make Default Term” 的准确含义

iTerm2 当前菜单文档描述的默认终端影响 `.command`、`.tool`、`.zsh`、`.csh`、`.pl` 等文件打开行为。[M1] 可访问的历史上游版本 `v3.4.2beta2` 在 `iTermLaunchServices.m` 中用 `public.unix-executable`、`kLSRolesShell` 和 `LSSetDefaultRoleHandlerForContentType` 设置默认 Shell role；该证据只代表历史实现，不能声称已核对其当前主分支。[M2]

没有找到 Apple 公开的 Windows 式通用 console delegation 接口。macOS 可以在日常入口替代 Terminal，但文件关联、URL 关联、Finder 菜单和第三方工具配置须分别完成。写死 `Terminal.app` 路径或使用其专有 AppleScript dictionary 的应用，不会自动兼容 Termeta；若将来支持 Apple Events 命令，需要独立的 scripting 契约。

### 4.2 注册和设置 API

| 目标 | 声明 / API | 注意事项 |
| --- | --- | --- |
| 声明可打开的文件 | Info.plist `CFBundleDocumentTypes`、`LSItemContentTypes`、`CFBundleTypeRole = Shell` | 声明能力不等于成为默认，且需要真实文件接收/执行语义 |
| 声明 URL scheme | `CFBundleURLTypes`、`CFBundleURLSchemes` | 可以先声明自有 `termeta:`；`ssh:`、`telnet:` 分别处理 |
| 设置 URL 默认应用 | macOS 12+ `NSWorkspace.setDefaultApplication(at:toOpenURLsWithScheme:completion:)` | 异步回调，可能由系统要求用户同意 |
| 设置内容类型默认应用 | macOS 12+ `NSWorkspace.setDefaultApplication(at:toOpen:completion:)` | 接受 UTType，没有旧接口的 role 参数 |
| 旧版关联 API | `LSSetDefaultRoleHandlerForContentType`、`LSSetDefaultHandlerForURLScheme` | Apple 已标记 Deprecated；是否兼容旧 macOS 取决于产品最低版本 |

依据 Apple Info.plist、NSWorkspace 文档及本机 SDK 头文件的静态核对。[M3][M4][M5] **现代 content type setter 是否与旧 `kLSRolesShell` 完全等价仍需原型验证。** 不提前批量设置 `public.unix-executable` 的泛化默认应用，避免在缺乏接收能力时改变过大范围。

设置应由明确用户操作触发，完成后重新查询系统实际关联，分别展示每一类状态；取消系统确认不能记为成功。

### 4.3 Finder 与命令行

iTerm2 官方 FAQ 的目录入口是 `Services > New iTerm2 Tab Here`。Apple 的 `New Terminal Tab at Folder` 是自带服务，不应假设更改默认关联后会转投第三方。[M6]

推荐先以 `NSServices` + `NSApplication.setServicesProvider:` 提供“在 Termeta 中打开终端”，服务方法从 pasteboard 读取所选目录 URL。服务可能在注册后立即到达，必须先准备 Rust 请求队列，再等待主窗口/解锁/前端就绪。[M7] Finder Sync 扩展主要面向同步状态和监控目录，不是首选通用终端菜单方案。[M8]

`open -a Termeta --args ...` 可以演示冷启动，但 Apple `NSWorkspace.OpenConfiguration.arguments` 的保证针对启动新实例。暖启动投递应使用自有 CLI + 单实例通道或系统 URL/file 事件，不能只依赖 `open --args`。[M9]

脚本文件打开不是普通目录打开。`.command`/`.tool` 需要单独规定可执行权限、shebang、cwd、退出后保留和系统隔离属性；不能在只支持打开 Shell 时就接管这些文件关联。

## 5. Linux：桌面启动器与多套选择机制

Linux 可以注册为终端模拟器，但没有统一覆盖全部桌面的默认终端 setter。`.desktop` 分类、默认 MIME 应用、Debian alternatives 和桌面自有选择并非同一机制；它们只影响使用相应解析器的调用者。

### 5.1 Desktop Entry 与 xdg-terminal-exec

基础注册使用下列元数据；可执行路径和图标名应由实际安装产物确定，不在运行时猜测：[L1]

```ini
[Desktop Entry]
Type=Application
Name=Termeta
Exec=termeta
TryExec=termeta
Icon=termeta
Terminal=false
Categories=System;TerminalEmulator;
```

`Terminal=false` 表示本 GUI 不需要由另一终端承载；`Terminal=true` 的含义不是“这是一个终端”。分类也不会自动改变用户的默认值。

上述是普通应用身份示例，暖启动可以仅聚焦。用于默认终端候选的入口必须保证无命令调用也创建 Local TAB，例如后续采用 `Exec=termeta --local` 的专用 desktop entry；执行参数出现时覆盖默认交互 Shell，不能额外创建一个 Shell TAB。实际 desktop ID 需与选择配置一致。

`xdg-terminal-exec` 是值得适配的跨桌面方向，但上游仍称其为 proposed specification，不能写成全部桌面已采用的正式统一接口。[L2] v0.11.0 改为 `[X-]TerminalArg*`，要求显式执行参数键；v0.12.0 加强 Desktop Entry 字符串/Exec 解析。建议实现 argv 执行后再声明：

```ini
X-TerminalArgExec=-e
X-TerminalArgDir=--working-directory
X-TerminalArgTitle=--title
```

`X-TerminalArgExec` 是命令前插入的单个参数；上游允许显式空值表示不需要前缀。引用实现对缺失字段回退 `-e` 是兼容行为，不应依赖。`AppId`、`Hold` 等可选键只在实现且验收后添加；路径和命令不能重拼成 Shell 字符串。

用户候选配置为 `${XDG_CONFIG_HOME:-~/.config}/xdg-terminals.list`，每行一个 desktop entry ID，优先项在前；还有由 `XDG_CURRENT_DESKTOP` 派生的 `${desktop}-xdg-terminals.list` 及系统回退位置。后续设置功能必须检测桌面专用列表优先级，不能只写通用文件就宣称生效。[L2]

上游调用形态如 `xdg-terminal-exec --dir=/path -- executable arg1 arg2`。终端未声明 Dir 键时，引用实现可能先切换 cwd 再启动，因此单实例投递必须携带**本次调用进程的 cwd**，不能沿用已运行主实例的 cwd。

### 5.2 Debian、GNOME、KDE 与文件管理器

| 环境 | 核实的机制 | 建议 |
| --- | --- | --- |
| Debian / 衍生发行版 | `x-terminal-emulator` alternatives；Policy 要求 VT100、`-e command [args…]` 及 `-T title`；命令语义类似直接 `execvp` | 先实现 CLI 兼容，再由 deb 安装/卸载脚本注册/移除自身候选与手册 slave；用户选择默认，不强制 `--set` |
| GNOME | 当前 `exec` / `exec-arg` gsettings schema 已标 deprecated and ignored；当前 GLib main 优先查找 `xdg-terminal-exec` 后再回退其他终端 | 不推荐写旧 gsettings；按实际发行版 GLib/桌面版本验证 |
| KDE Plasma / KIO | 当前 launcher 读取 General 的 `TerminalService`（desktop ID）和 `TerminalApplication`（后备可执行项），再构造 `-e` 等参数 | 提供 Plasma 默认应用选择指引，不能假定只改 XDG 列表就覆盖 |
| GNOME Files / Nautilus | 当前 main 的目录终端入口经 D-Bus 定位 `org.gnome.Console` | 需独立的 Termeta 菜单/扩展方案，不能冒充 Console 服务名 |

来源：[L3][L4][L5][L6]。这些是当前上游行为，不代表所有已发布发行版。AppImage 不应自动修改系统级 alternatives；文件位置移动后，其 CLI/desktop 注册需要修复。`.desktop` 的 `x-scheme-handler/...` 只解决协议关联，不是通用默认终端。

## 6. Tauri 适配与依赖结论

本机 registry 中静态核对了已锁定 single-instance 2.4.2 和 Tauri 2.11.5 源码：回调为 `Vec<String>` argv 与 `String` cwd；macOS 实现也能转交直接执行二进制的后续实例参数。Tauri 的 `RunEvent::Opened { urls }` 和 macOS `Reopen` 是不同事件：Opened 处理文件/URL，Reopen 默认只恢复窗口，不重复触发冷启动本地 TAB。[T1][T2]

不能无损地用当前字符串回调传输任意 Unix 非 UTF-8 路径；实施需明确拒绝或设计原生路径编码通道，不允许有损替换后打开错误目录。CLI 优先用 `args_os` 解析，新增 launcher 的传输契约需与接收端一致。

官方 deep-link 插件是后续 URL 入口候选，当前未引入。Windows/Linux 可与 single-instance 的 `deep-link` feature 联动；macOS 需打包时声明 scheme，不能照搬运行时注册方式。插件提供传输，不证明请求可信；冷启动和后续事件可能重叠，必须去重。[T3]

在线最新指南已列出较高 Rust 最低版本要求，而仓库锁定的是另一组具体版本。后续新增插件应重新核对固定 release 的 API、features、MSRV、许可证和公告；本轮不新增依赖、不改锁文件。

## 7. 推荐范围与未决项

| 顺序 | 交付能力 | 进入下一阶段的门槛 |
| --- | --- | --- |
| A | 设置“启动进入保险箱 / 本地终端”，解锁后只创建一次 | 普通启动、解锁失败重试、设置热应用不多开、外部请求优先 |
| B | CLI `--local` / cwd、暖启动转交、统一启动请求队列 | cwd/Unicode/并发/配额/失效请求；明确不支持的原生路径 |
| C | macOS Services、Linux desktop 身份、Windows 自有菜单 | 安装/升级/卸载、文件管理器/桌面矩阵；注册不抢默认 |
| D | 显式 CLI argv 与 Linux 默认启动器适配；URI、SSH/Telnet 默认关联及 macOS 脚本文件关联分别验收 | Linux 执行参数兼容；来源与参数边界、主机信任、脚本确认；不能借 URI 执行任意命令 |
| E | Windows 系统默认终端独立 POC，成功后再生产化 | 包身份/双扩展/COM/console host、inbound 生命周期、提权、失败及卸载回退 |

还需决策：产品最低系统版本；是否必须在保险库解锁前使用 Local；Linux 首批桌面/发行版；Windows 包装及 console host 分发版本；macOS 旧 Shell role 与现代 UTType 的行为。第一阶段保留现有解锁边界，无需等待这些研究项全部关闭。

## 8. 资料来源

以下均于 2026-10-17 查阅。官方文档用于接口语义；上游源码用于解释实现，不能外推成全部版本的兼容承诺。

- W1：[Microsoft Windows Terminal 安装与默认终端支持](https://learn.microsoft.com/en-us/windows/terminal/install)。
- W2：[Windows Terminal 默认终端设计](https://github.com/microsoft/terminal/blob/main/doc/specs/%23492%20-%20Default%20Terminal/spec.md)。
- W3：[IConsoleHandoff IDL](https://github.com/microsoft/terminal/blob/main/src/host/proxy/IConsoleHandoff.idl)、[ITerminalHandoff IDL](https://github.com/microsoft/terminal/blob/main/src/host/proxy/ITerminalHandoff.idl)、[当前接收实现](https://github.com/microsoft/terminal/blob/main/src/cascadia/TerminalConnection/CTerminalHandoff.h)。
- W4：[Package manifest](https://github.com/microsoft/terminal/blob/main/src/cascadia/CascadiaPackage/Package.appxmanifest)、[DelegationConfig](https://github.com/microsoft/terminal/blob/main/src/propslib/DelegationConfig.cpp)。
- W5：[Tauri Windows Installer](https://v2.tauri.app/distribute/windows-installer/)。
- W6：[Microsoft winapp CLI 与 Tauri](https://learn.microsoft.com/en-us/windows/apps/dev-tools/winapp-cli/guides/tauri)。
- W7：[为非打包应用授予身份](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/grant-identity-to-nonpackaged-apps)。
- W8：[将 ConPTY NuGet 产品化为受支持第三方包的议题 #15065](https://github.com/microsoft/terminal/issues/15065)。
- W9：[Explorer 现代菜单集成](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/integrate-packaged-app-with-file-explorer)。
- W10：[Shell 静态菜单](https://learn.microsoft.com/en-us/windows/win32/shell/context-menu-handlers)、[URI 激活](https://learn.microsoft.com/en-us/windows/apps/develop/launch/handle-uri-activation)。
- M1：[iTerm2 菜单](https://iterm2.com/documentation-menu-items.html)。
- M2：[iTerm2 历史关联实现，v3.4.2beta2](https://gitlab.com/gnachman/iterm2/-/blob/v3.4.2beta2/sources/iTermLaunchServices.m?ref_type=tags)。
- M3：[Apple Core Foundation Info.plist Keys](https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Articles/CoreFoundationKeys.html)。
- M4：[NSWorkspace URL 默认应用](https://developer.apple.com/documentation/appkit/nsworkspace/setdefaultapplication(at:toopenurlswithscheme:completion:))、[内容类型默认应用](https://developer.apple.com/documentation/appkit/nsworkspace/setdefaultapplication(at:toopen:completion:))。
- M5：[旧文件 role setter](https://developer.apple.com/documentation/coreservices/1444955-lssetdefaultrolehandlerforconten)、[旧 scheme setter](https://developer.apple.com/documentation/coreservices/1447760-lssetdefaulthandlerforurlscheme)。
- M6：[iTerm2 FAQ](https://iterm2.com/faq.html)、[Apple 从 Finder 打开 Terminal](https://support.apple.com/en-mn/guide/terminal/trmlb20c7888/mac)。
- M7：[Apple 提供 Services](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/SysServices/Articles/providing.html)。
- M8：[Apple Finder Sync](https://developer.apple.com/library/archive/documentation/General/Conceptual/ExtensibilityPG/Finder.html)。
- M9：[Apple OpenConfiguration](https://developer.apple.com/documentation/appkit/nsworkspace/openconfiguration)。
- M10：[iTerm2 自有 URL scheme 与命令确认](https://iterm2.com/documentation-url-scheme.html)。
- L1：[Desktop Entry keys](https://specifications.freedesktop.org/desktop-entry-spec/latest/recognized-keys.html)、[TerminalEmulator 分类](https://specifications.freedesktop.org/menu-spec/latest/category-registry.html)。
- L2：[xdg-terminal-exec README 与版本说明](https://github.com/Vladimir-csp/xdg-terminal-exec/blob/master/README.md)、[规范提议](https://gitlab.freedesktop.org/terminal-wg/specifications/-/merge_requests/3/diffs)。
- L3：[Debian Policy 11.8.3](https://www.debian.org/doc/debian-policy/ch-customized-programs.html#packages-providing-a-terminal-emulator)。
- L4：[GNOME 旧终端设置 schema](https://github.com/GNOME/gsettings-desktop-schemas/blob/master/schemas/org.gnome.desktop.default-applications.gschema.xml.in)、[GLib GIO launcher](https://github.com/GNOME/glib/blob/main/gio/gdesktopappinfo.c)。
- L5：[KIO terminal launcher](https://github.com/KDE/kio/blob/master/src/gui/kterminallauncherjob.cpp)。
- L6：[Nautilus 目录操作](https://github.com/GNOME/nautilus/blob/main/src/nautilus-files-view.c)、[D-Bus launcher](https://github.com/GNOME/nautilus/blob/main/src/nautilus-dbus-launcher.c)。
- T1：[single-instance 2.4.2 固定源码](https://docs.rs/crate/tauri-plugin-single-instance/2.4.2/source/)。
- T2：[Tauri 2.11.5 固定源码](https://docs.rs/crate/tauri/2.11.5/source/src/app.rs)。
- T3：[Tauri Deep Linking](https://v2.tauri.app/plugin/deep-linking/)、[Single Instance](https://v2.tauri.app/plugin/single-instance/)。

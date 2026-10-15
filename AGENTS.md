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
| Tauri 2 + Vue 3 + TypeScript | 已替换 greet 模板为模块化 SSH 工作区，代码实现待集成验证 |
| 首期 M1/M2 | core/ssh/storage/security、桌面命令、主机管理、多标签、认证、设置已落源码；未通过运行验收 |
| 终端生命周期 | TerminalRegistry 按 terminalId 保活；断线/失败保留；重连排空旧流后追加；全屏/IME/三平台待验证 |
| UI、快捷键、配色 | 已接入 Reka UI、xterm 与 Pinia；快捷键编辑、8 内置配色、自定义 JSON 和热应用已实现待验证 |
| 存储与凭据 | SQLx SQLite migration、主机/分组/信任/历史/app 设置；OS keyring，禁止明文回退；实际系统服务待验证 |
| 安全边界 | 自定义命令 AppManifest/ACL、main owner 校验、导航限制与 CSP 已落配置；不表示已通过安全审计 |
| Local / Serial / SFTP / 隧道 / 分屏 | 保留设计，未实现，未暴露可用入口 |
| 验证 | 用户提供 macOS 启动日志：进入 setup 后因迁移校验和不匹配失败；已修源码待复验。代理仍仅静态检查，禁止主动编译/运行 |

已移除 greet、移动端入口宏和宽泛默认 opener 权限；原生私钥选择只返回引用，凭据接口不返回秘密。根 Cargo workspace 统一四个领域 crate 与 Tauri 壳，锁文件在仓库根目录。代理没有启动应用、连接真实主机或执行 Rust 编译/测试；用户已尝试 macOS 启动但尚未成功进入工作区，三平台验证未完成。

## 4. 架构约定

当前已建立 `src/modules/`、`src/services/desktop/`、`src-tauri/src/commands/` 及 core/ssh/storage/security 四个 crate。完整目标结构见 [架构文档](docs/03-architecture.md)，后续领域仍是计划。

- `src/modules/*`：hosts、terminal、workspace、settings，以及后续 files/transfers；Vue/Pinia 只保存轻量状态，不保存终端输出或文件正文字节。CommandRegistry/KeymapService 统一动作与快捷键，TerminalThemeService 独立管理终端配色。
- `src/services/desktop/`：唯一 Tauri IPC 门面；组件不直接调用 `invoke`。
- `src-tauri/src/`：命令 DTO、窗口所有权校验、Channel 桥接、依赖组装；不承载协议核心逻辑。
- `crates/terminal-core/`：不依赖 Tauri/Vue/russh 的会话模型、能力、actor 控制、生命周期和扩展协议。
- `crates/terminal-ssh/`：russh、主机密钥校验、认证、PTY/Shell、KeepAlive；依赖 core，通过接口使用凭据与信任仓库。
- `crates/terminal-storage/`、`crates/terminal-security/`：SQLite 与操作系统凭据存储适配器。核心定义端口，壳负责注入。
- Local/Serial 的工厂与配置扩展点先保留；具体 crate 在开发对应功能时再创建。SFTP/隧道通过 SSH 连接服务扩展，不塞进通用终端接口。M3 的 `terminal-files` 定义文件服务/传输/冲突端口，`terminal-sftp` 和本地文件适配器实现；本地浏览不依赖 Local PTY，独立 SFTP 不启动 Shell。

会话采用**每会话 actor + 有界字节队列 + 独立取消信号**；重连创建新的后端 session ID，但保留同一 Tab、terminalId 与 xterm 实例，在原缓冲中追加输出。断线、连接失败和后端终态均不得自动关闭 Tab、清空或销毁终端；历史继续可滚动、选择、复制和搜索。TerminalRegistry 按 terminalId 持有实例，不能以 sessionId 变化触发重建。输出走二进制 Channel，控制事件走类型化 Channel，使用消费 ACK 实现端到端背压。所有凭据、信任与网络操作由 Rust 管理。

前端控件采用 **Reka UI primitives + 项目封装组件 + CSS 变量/自定义样式** 的方向。终端宿主与工作区生命周期由应用掌控；Reka 的隐藏/卸载行为不得销毁终端。release 已锁定，三平台集成尚未验证，详见 UI 选型文档。

快捷键设置分应用内全局与终端类别（含修饰键点击链接）；系统后台热键另行 opt-in。Cmd+L 的 Local 动作待 M4 工厂可用才启用。终端配色独立于应用主题，至少 8 个内置方案，支持任意取色与 JSON 导入，热切换不清历史。SFTP 左/右/底部可隐藏面板与双栏 Tab 共用后端，远端互传默认有界内存中转，上传/下载/互传均检查覆盖并确认；原生拖出必须单独验证，不能用 Web DnD 替代。

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
| [13-ui-stack.md](docs/13-ui-stack.md) | Reka UI 官方预研、控件映射、桌面适配与集成边界 |
| [14-shortcuts.md](docs/14-shortcuts.md) | 全局/终端快捷键、修饰键点击、冲突/焦点/配置与原生边界 |
| [15-terminal-themes.md](docs/15-terminal-themes.md) | 内置/自定义配色、取色编辑、导入导出与热应用 |
| [17-implementation-status.md](docs/17-implementation-status.md) | 首期源码交付、静态验证证据、已知限制与运行验收入口 |
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

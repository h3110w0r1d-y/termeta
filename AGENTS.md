# Termeta 开发协作指南

更新：2026-10-27。适用于整个仓库，文档和代码注释默认中文。功能状态以 [产品需求](docs/01-product-requirements.md) 为准，不在本文重复清单或追加修复日志。

## 1. 项目与阅读入口

Termeta 是 Tauri 2 + Rust + Vue 3 + TypeScript 桌面终端客户端，目标 Windows、macOS、Linux，不计划 Android/iOS。以 Termius 桌面核心体验为参照，不复制品牌或专有实现；用户指定的 SSH 系统/类型图标素材是明确例外。

开发前阅读本文、目标模块文档及 [路线](docs/10-roadmap.md)。完整索引见 [docs/README.md](docs/README.md)；关键设计取舍见 [架构](docs/03-architecture.md)，本机数据恢复见 [存储](docs/07-storage.md)。

## 2. 执行与数据保护

- **不要主动构建、运行或启动项目。** 未得到用户明确要求时，不执行 `pnpm dev/build/preview`、`pnpm tauri dev/build`、`cargo build/run/check/test/clippy`，不通过 IDE、脚本、测试、CI 或依赖生命周期间接触发编译、启动或打包。`cargo check`、Rust 测试同样会编译。
- 允许按任务安装/更新依赖、下载工具、更新锁文件及执行不构建/运行项目的静态检查，无需另行确认；安装须禁用会触发项目构建/运行的生命周期脚本。不主动启动测试服务器、连接真实主机或串口。
- 可阅读源码/官方资料、检查差异、格式及文档链接。交付报告如实说明本轮检查范围，不为满足清单擅自执行命令；检查结果不写入项目文档。
- 保留用户已有修改；不自行提交、推送、发布或创建真实凭据。仅写文档不创建空 crate、添加无关依赖或改业务模板。
- **已实际应用的数据库迁移（含开发环境）必须保持原始字节。** 后续变更新增迁移，不跳过 SQLx 校验或改写 `_sqlx_migrations`。故障先保留加密数据库及配套 vault 元数据；迁移 4 保持原始字节，索引调整由迁移 5 完成，见 [恢复说明](docs/07-storage.md#9-已应用迁移校验和不一致的恢复)。

## 3. 架构约束

- `src/layout/` 装配保险库与外观，App 保留根布局和稳定终端宿主，设置由独立 SettingsWindow 装配；公共基础样式在 `src/assets/base.css`。`src/modules/` 按业务分责，Vue/Pinia 只保存轻量状态，不保存终端输出、文件/备份正文或转发载荷。sessions 模块拥有通用控制投影/挑战，Workbench 只管布局/焦点；关闭归档统一走 TerminalRegistry。
- `src/services/desktop/` 是唯一 Tauri IPC 门面，组件不能直接 invoke；CommandRegistry 统一按钮/菜单/快捷键。KeymapService/TerminalThemeService 是设计职责名，不假定有同名类。
- 壳的 `commands/` 负责 DTO、窗口/资源所有权、Channel 和编排，`files/` 负责文件服务，协议核心留在 crate。九个领域 crate 为 core、ssh、telnet、local、serial、storage、security、files、sftp；core 不依赖 Tauri/Vue/具体协议库。security 当前直接依赖 storage，云 provider 位于壳的 backup 模块。
- 每会话 actor、有界字节队列和独立取消；输出二进制 Channel、控制类型化 Channel，消费后 ACK。全局锁不跨网络/数据库/Channel 等待，ACK 不得阻塞关闭、KeepAlive 或信任响应。凭据、信任、网络与设备由 Rust 管理。
- terminalId 跨重连稳定，sessionId 每次连接更新；TerminalRegistry 持有 xterm。断线/失败/终态保留标签、实例与历史，只有用户关闭视图才 dispose；重连隔离旧流，不自动重放输入。
- Workbench 是整个工作台，TerminalTabGroup 是运行标签组，Workspace 专指可保存的工作区。useWorkbenchStore 分离 terminals/groups，activeId 指固定页、终端组或日志标签，activeTerminal 指聚焦终端；布局树计算矩形，TerminalStage 以稳定 key 承载终端，不因排序、分屏、交换或迁移重建。
- 终端、文件、转发分别使用 sessions/file_sessions/forward_sessions；独立文件/转发不申请 PTY/Shell 或执行终端启动命令。通用连接租约、完整 TransferManager 尚未实现。

## 4. 实现规则

- 前端使用 pnpm、Vue 3 Composition API、严格 TypeScript，Rust edition 2024；不切 React，桌面本地架构不引入 HTTP/WebSocket 后端。依赖落地或升级须复核具体 release 的 API/features/MSRV/许可/公告，同步锁文件及 [依赖说明](docs/02-library-research.md)，不以 latest 页面锁版本。
- **新增或修改的代码注释使用中文**，保留标识符/API 和必要术语，说明意图与边界，避免复述代码。
- Vue/TS 跨越两层及以上的相对导入使用 `@/`（`src/`）别名；仓库根目录的资源清单使用 `@resources/`（`resources/`），同目录与单层父目录可保留相对导入。
- 通用控件优先 Naive UI 2.45.3，封装只做薄适配，使用公开 props/themeOverrides，不复制库皮肤。base.css 只保留主题、基础规则与跨业务共享布局；组件样式使用 scoped，动态 DOM 用明确 :deep()，必须全局生效的适配由所属业务入口导入并限定语义类名，禁止全局覆盖控件库通用类。应用标签使用 AppTabs。
- NForm model、控件 v-model 与 rules/path 必须共享响应式字段，禁止对象字面量/展开/computed 快照作 model。已有 ref 可用 reactive 包装，不能复制 .value 后异步同步；嵌套调整同步 rules/path。程序更新通过 FormItem 公开 validate 重验，保留正常 input/change/blur 及提交校验，不用清错、取消校验或 nextTick 补丁掩盖旧值。可选字段不强制必填，弹窗操作放 AppDialog #footer。
- Vue 沿用仓库 Prettier 配置，htmlWhitespaceSensitivity=ignore 仅限 *.vue；多行正文、闭合标签及相邻组件保持清晰换行。修改 Vue 后执行 `pnpm format:vue`、`pnpm check:format:vue`。精确空格使用显式文本，必要时单节点 prettier-ignore 并说明，保留 pre 内容。
- 通用 UI 图标使用 @tabler/icons-vue；不新增手写 SVG/字符替代。用户指定的 28 个 SSH 系统 SVG 见 resources/host-system-icons.json，主机系统标志保留原样；原生窗口和品牌资源保留平台/品牌形式。AssetIcon 仅作语义映射。图标按钮有可访问名称，装饰图标 aria-hidden。
- 界面文案用 src/i18n 稳定语义 key，维护 zh-CN/zh-TW/en-US/ja-JP/ko-KR；动态整句用类型化命名参数，动态 key 用 PlainMessageKey，校验传 key 而非译文，选项/校验保持响应式。用户内容、后端诊断、远端输出保留原文；提示说明操作和后果，不展示内部实现。执行 `pnpm check:i18n`；表单与边界检查用 `pnpm check:structure`。
- 自定义命令须加入 AppManifest::commands、handler 和 capability，并校验本地窗口及资源 owner；只写 capability 不足以保护命令。禁止默认接受 SSH 密钥、明文存秘密、默认弱算法或自动执行远端输出。

## 5. 领域行为约束

- 主机分组前后端防环；删除默认保留上移，递归删除独立二次确认，Rust 单事务校验范围/revision，保留公共凭据与已有会话快照。即时偏好串行局部保存，旧设置草稿不能覆盖。搜索不改分组范围，批量打开包含后代。
- 固定保险库/SFTP 不可关闭、不创建终端；设置为独立单实例窗口，权限仅限设置、专用改密/JSON 导出及备份，通用 session owner 仍只 main。模态/录制/弹层优先处理快捷键，弹窗 Cmd+W 不关闭标签或主窗口；认证弹窗退出后恢复当前活动终端焦点。
- 广播默认关闭、仅内存；成员重组关闭，排序/比例/子树交换保持。排除命令窗格、点击/拖动、设备应答和认证；键盘/粘贴要求焦点，远端滚轮可来自当前组非聚焦窗格，只分发源端最终动作，不合成 wheel 或在目标累积。inputOrigin 集中适配固定 xterm，能力缺失拒绝广播；目标独立编码、限额、代次检查，不重放。
- 标准粘贴放行系统 paste，由宿主执行大小/多行/广播校验；自定义绑定保留显式剪贴板路径。应用内快捷键与系统后台热键分开。完整路由见 [快捷键](docs/14-shortcuts.md)。
- SFTP 侧栏仅单 SSH 终端且与分屏互斥，隐藏保留任务，断开/关闭取消，重连不重放。双栏独立连接与对拷，不借原生拖出做内部复制；文件正文仅 Rust 流动，上传确认覆盖/目录合并不承诺回滚，下载拒绝覆盖。
- 原生拖入仅消费 owner-bound 一次性 dropId，不接收任意本地路径；macOS 承诺拖出兑现请求后才显示任务，取消清理手势与迟到请求。限额与路径边界见 [SFTP](docs/16-sftp-workspace.md)。
- SSH 主要认证为密码或公钥，Keyboard-interactive/OTP 依服务器协商且 OTP 不保存；保存的交互 Shell SSH 系统识别仅用固定只读命令、独立 exec，不写 PTY/广播。转发默认回环，SOCKS5 仅回环/TCP CONNECT，独立取消，不持久化运行状态或自动启动。
- 本机保险库独立初始化与解锁；Gist/WebDAV 在已解锁后的设置中使用独立备份密码，上传独立版本、选择恢复版本后验证并新增合并，无二次确认。云包只含白名单业务数据，设置/会话日志/云秘密不导出，不用第三方包绕过既有库解锁。边界见 [备份](docs/22-data-backup.md)。
- 应用主色与终端配色独立，热应用不清历史、不重连；字体限内置 12 家族。其他界面约束见 [桌面交互](docs/09-desktop-ux.md)，布局/存档契约见 [工作区](docs/18-terminal-workspaces.md)，资源与许可见 public/fonts 和 resources。

## 6. 文档同步与交付

功能、架构、接口、数据、安全、依赖或范围变化时，同轮更新需求状态、后续路线与相关领域文档，必要时更新本文。关键决策保留理由和影响；接口冲突修订 Rust/TS 契约，不另建第二套接口。

增删文档同步索引和本地链接。文档只保留当前行为、有效约束、兼容/恢复说明及实际未实现项；字段/常量/组件流程留在源码，不保留实现日志、测试/验证/验收状态或逐轮检查记录。功能以源码为准：完整实现标记 `[x]`，未实现或部分实现保留 `[ ]` 并说明余项，不以是否测试或验证决定完成状态。

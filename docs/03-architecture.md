# 系统架构与模块边界

更新：2026-10-19。本文描述当前实现及其设计取舍；功能状态见 [产品需求](01-product-requirements.md#8-当前实现与限制)。

## 1. 分层与代码入口

| 层 | 入口 | 职责 |
| --- | --- | --- |
| 界面 | `src/layout/`、`src/modules/` | 页面装配、轻量状态、终端布局和用户操作 |
| 前端服务 | `src/services/desktop/`、`src/services/commands/` | 唯一 IPC 门面；统一按钮、菜单和快捷键命令 |
| 桌面壳 | `src-tauri/src/commands/`、`state.rs`、`lib.rs` | DTO 校验、窗口/资源授权、Channel、依赖装配与原生交互 |
| 会话核心 | `crates/terminal-core/` | 协议无关模型、端口、会话 actor、有界队列、取消与 ACK |
| 协议适配 | `terminal-ssh`、`terminal-telnet`、`terminal-local`、`terminal-serial` | SSH、Telnet、本地 PTY 和串口驱动 |
| 数据与安全 | `terminal-storage`、`terminal-security` | SQLCipher、迁移、凭据、保险库、白名单备份与加密 |
| 文件 | `terminal-files`、`terminal-sftp`、壳的 `files/` | 文件端口、有界复制、SFTP 浏览与传输编排 |
| 云备份 | 壳的 `backup/`、`backup_runtime.rs` | Gist/WebDAV provider、操作预算和预览令牌 |
| 官方同步 | 壳 `official_sync`、security 主密钥、storage 同步合并 | 双模式门禁、同主密钥整包、账号隔离基线、稳定 ID 三方合并与 CAS |
| 官方账号 | `src/modules/account/`、壳的 `account/` | 账号摘要与浏览器入口、固定HTTPS服务访问、加密会话及浏览器授权；不耦合终端核心 |

网络、设备、信任、数据库及保存的秘密均由 Rust 管理。Vue/Pinia 仅保存元数据，不保存终端输出、文件正文、备份数据库正文或转发载荷。xterm 实例和缓冲由终端注册表管理。

## 2. Rust 依赖方向

```text
src-tauri → core / ssh / telnet / local / serial / storage / security / files / sftp
ssh / telnet / local / serial / storage / files → core
security → core + storage
sftp → core + files + ssh
```

九个领域 crate 共用根 Cargo workspace、resolver 3 和 edition 2024。core 不依赖 Tauri、Vue、SQLx 或具体协议库。壳将凭据、信任、系统识别等端口实现注入适配器。security 当前直接依赖 storage 协调凭据和保险库事务，不能将完全反转的存储依赖写成已有实现。

## 3. 会话与终端生命周期

壳分别持有 `sessions`、`file_sessions`、`forward_sessions` 三个 SessionManager。终端使用四种协议工厂；文件与转发使用专用 SSH 工厂，复用认证/信任，不申请 PTY/Shell，也不执行主机的终端启动命令。资源归属、挑战响应和关闭按所属 manager 路由。

每会话 actor 持有驱动，有界队列限制数据，独立取消信号保证输出背压不阻塞关闭。全局表锁只查询或复制轻量句柄，不跨网络、数据库或 Channel 等待。详见 [会话](04-session-model.md) 与 [IPC](06-ipc-and-streaming.md)。

`terminalId` 标识跨重连稳定的视图，`sessionId` 标识一次后端连接。断线、失败和远端退出释放连接，保留终端与历史；用户关闭视图时先排空尾输出并将 xterm 现存文本归档为本机加密会话日志，提交成功才销毁实例。重连排空旧流、清理未发送输入并隔离迟到回调，然后向同一终端绑定新会话。

Workbench 表示整个工作台，TerminalTabGroup 表示运行标签组，Workspace 表示多终端工作区及其显式存档。`useWorkbenchStore()` 分离终端和标签组；TerminalStage 以稳定 terminalId 承载终端，布局树只决定几何位置。标签切换、分屏、交换和迁移不重建终端。存档只保存布局与连接引用，打开创建新会话，不自动恢复退出前标签。详见 [工作台与工作区](18-terminal-workspaces.md)。

## 4. 数据、设置与权限

保险库解锁后才允许业务操作；全新 setup 的云端初始化有独立授权，已有 locked 库不能使用。storage 完成迁移与外键检查后发布数据库池。改密通过可切换连接池和数据库生命周期门闩协调加密副本及元数据切换，不重建活动终端。格式与故障恢复见 [存储](07-storage.md)。

设置使用独立单实例窗口。Rust 串行合并局部设置并发出失效通知，各窗口重读后热应用；即时偏好不能被旧设置草稿覆盖。settings 窗口仅获得设置、专用改密、JSON 导出、备份和官方账号权限，通用会话所有权仍仅 main。ACL、来源和资源归属共同构成授权，见 [安全](08-security.md)。

备份分为 storage 的白名单快照/合并、security 的压缩/加密、壳的云 provider/编排三层。provider 只处理不透明包；增加云厂商不改变终端核心。第三方仅用于离线模式，官方模式采用独立的主密钥/合并编排。详见 [第三方备份](22-data-backup.md) 与[官方同步](26-official-vault-sync.md)。

## 5. 文件与转发边界

单 SSH 终端侧栏通过该终端连接申请 SFTP 子系统，生命周期跟随终端；固定双栏使用两个独立文件会话。端口转发使用独立转发会话。文件正文和转发载荷不经过 WebView；文件命令及进度不复用终端帧/ACK。通用连接租约、本地文件 provider 和完整 TransferManager 尚未实现。详见 [SFTP](16-sftp-workspace.md) 与 [转发](21-port-forwarding.md)。

原生文件授权与平台拖放适配位于壳，应用内布局和对拷由前端交互处理。原生拖入使用绑定 owner 的一次性引用，macOS 拖出使用文件承诺，不能将 Web 拖动等同于系统拖出。

## 6. 关键设计取舍

| 决策 | 理由与代价 |
| --- | --- |
| Tauri + Rust + Vue，协议核心独立 | 桌面权限集中于壳，复用会话通路；桌面本地不引入 HTTP/WebSocket 后端，官方服务独立部署 |
| 消费后 ACK 与有界字节队列 | 防止 WebView 无限制积压；满额时允许反压远端输出，取消独立运行 |
| 稳定终端与布局/连接分离 | 保留历史和视图；需要旧流排空及代次隔离，不能用清屏规避 |
| 全库加密、OS 仅缓存主密钥 | 数据文件与秘密统一保护；缓存失败用密码恢复，数据库和元数据必须配对 |
| 改密在加密副本上执行 | 可恢复地切换数据库与元数据；需要额外磁盘空间及短暂数据库等待 |
| 文件/转发使用专用连接 | 不依赖终端 Shell；代价是尚无通用连接复用租约 |
| 白名单逻辑快照与非覆盖导入 | 排除本机设置并支持压缩；重新映射 ID，重复导入可能产生副本 |
| Naive UI 薄适配，应用主题与终端配色独立 | 通用控件沿用库能力，终端布局由应用管理，热应用保留实例 |

前端规则见 [桌面交互](09-desktop-ux.md) 和 [协作指南](../AGENTS.md)；依赖依据见 [依赖说明](02-library-research.md)。

## 7. 可选官方服务

2026-10-19 新增独立Python/FastAPI官方服务项目，账号、密文备份与分发更新控制面不进入桌面IPC或协议核心。客户端接入及服务端部署尚未验收，临时SSH远程协助仅规划，见[官方服务](24-official-service.md)。

官方管理端为独立Vue/Naive UI应用，HTTP服务层与auth/releases领域模块分离，通过同源/api反代接入服务端，管理员权限由后端最终校验；不接入桌面IPC。

官方账号由 Vue 账号领域、Rust 账号运行时与独立 accounts/oauth 服务端协作，三种登录方式统一系统浏览器、S256 proof 和一次性确认码兑换。邮箱表单与基础账号管理移至独立用户端，桌面会话永久可撤销；setup阶段Token只在Rust内存暂存，保险库可用后写SQLCipher。壳 `account/deep_link` 只复用官方协议注册，原生回跳直接匹配本机在途授权，不广播URI。见[账号认证](25-account-authentication.md)。

2026-10-19 决策：启动区分官方与本地/离线模式。官方云数据先通过保险库密码认证，解锁本地后对齐主密钥；全量白名单快照保留稳定 ID，以已观测云快照为基线做三方合并，CAS防止并发覆盖。本地模式保留独立密码的第三方手动备份，删除其首次启动恢复流程。前端只保留状态，合并/主密钥/云正文均留Rust，协议核心与终端实例不受模式切换影响。完整边界见[官方同步](26-official-vault-sync.md)。

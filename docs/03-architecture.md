# 系统架构与模块边界

更新：2026-10-27。本文维护当前分层、依赖方向与设计取舍；功能状态见 [产品需求](01-product-requirements.md)，剩余结构问题见 [重构评估](27-architecture-refactoring.md)。

## 1. 分层与代码入口

| 层 | 入口 | 职责 |
| --- | --- | --- |
| 界面与窗口 | `src/App.vue`、`src/layout/`、`src/modules/` | App 保留根布局和稳定终端宿主；layout 装配导航、连接、退出与外观；modules 按业务管理展示和操作 |
| 主题 | `src/theme/` | 共享配色模型、继承解析、导入校验、颜色运算与 xterm 映射，不依赖业务模块 |
| 前端服务 | `src/services/desktop/`、`src/services/commands/` | 唯一原生 API/IPC 门面；统一按钮、菜单和快捷键命令 |
| 桌面壳 | `src-tauri/src/commands/`、`state.rs`、`lib.rs` | DTO 校验、窗口/资源授权、Channel、依赖装配与原生交互 |
| 会话核心 | `crates/terminal-core/` | 协议无关模型、端口、会话 actor、有界队列、取消与 ACK |
| 协议适配 | `terminal-ssh`、`terminal-telnet`、`terminal-local`、`terminal-serial` | SSH、Telnet、本地 PTY 和串口驱动 |
| 数据与安全 | `terminal-storage`、`terminal-security` | SQLCipher、迁移、凭据、保险库、白名单备份与加密 |
| 文件 | `terminal-files`、`terminal-sftp`、壳的 `files/` | 文件端口、有界复制、SFTP 浏览与传输编排 |
| 云备份 | 壳的 `backup/`、`backup_runtime.rs` | Gist/WebDAV provider、操作预算和恢复令牌 |

网络、设备、信任、数据库及保存的秘密由 Rust 管理。Vue/Pinia 保存轻量投影，不保存终端输出、文件/备份正文或转发载荷。xterm 实例和缓冲归终端 Registry/runtime，Workbench Store 只管理终端展示、标签与布局，工作区存档校验和布局映射独立于 Store。

`src/modules/sessions/` 统一终端、文件和转发的控制投影、状态展示、终态判断、挑战与心跳登记，不持有连接资源，也不反向依赖具体业务或窗口布局。`src/services/desktop/` 集中协议 DTO 与原生 API 适配，组件不直接调用 Tauri。

前端保存项归 hosts，以共享元数据和 SSH/Telnet/Serial/Workspace 判别联合表达；desktop 在兼容 wire DTO 与领域类型之间集中转换。Rust 持久化/公开 DTO 仍采用扁平配置与字符串 kind，业务入口负责校验必需字段与协议适用性；这部分类型收敛尚未完成。

窗口导航、内容和终端宿主分开装配，顶部/左侧导航切换不移动 TerminalStage。共享主题不依赖 Registry；根外观、终端渲染和局部控件各自在消费边界应用配色，热应用复用现有实例。用户可见规则见 [桌面交互](09-desktop-ux.md)。

业务页面保留编排，主机列表呈现、分组编辑/删除弹窗、文件表格手势和终端样本预览各归独立职责。组件同时拥有其展示样式，跨组件的动态 DOM 适配由拥有该行为的入口加载；全局 base.css 不维护业务规则。

## 2. Rust 依赖方向

```text
src-tauri → core / ssh / telnet / local / serial / storage / security / files / sftp
ssh / telnet / local / serial / storage / files → core
security → core + storage
sftp → core + files + ssh
```

九个领域 crate 共用根 Cargo workspace、resolver 3 和 edition 2024。core 不依赖 Tauri、Vue、SQLx 或具体协议库；壳向适配器注入凭据、信任和系统识别端口。security 直接依赖 storage 协调凭据与保险库事务，当前不引入额外仓储抽象。

## 3. 会话与终端生命周期

壳分别持有 `sessions`、`file_sessions`、`forward_sessions` 三个 SessionManager。终端使用四种协议工厂；独立文件和转发使用专用 SSH 工厂，复用认证/信任，不申请 PTY/Shell，也不执行终端启动命令。

每会话 actor 持有驱动，有界字节队列限制积压，独立取消与挑战通路避免输出背压阻塞关闭或认证。全局表锁只查询或复制轻量句柄，不跨网络、数据库或 Channel 等待。驱动结束或请求关闭时，core 通过窄撤销回调和独立令牌通知壳取消附属文件资源，正常结束仍可排空终端尾输出。详见 [会话](04-session-model.md) 与 [IPC](06-ipc-and-streaming.md)。

`terminalId` 跨重连稳定，`sessionId` 每次连接更新。断线、失败和远端退出保留终端与历史；重连隔离旧流，不重放输入。关闭标签/窗格及窗口退出共用 Registry 入口，等待结束与归档成功后才销毁实例，失败保留视图供重试，见 [日志契约](23-snippets-and-session-logs.md#3-关闭与退出)。

Workbench 是整个工作台，TerminalTabGroup 是运行标签组，Workspace 是多终端工作区及其显式存档。TerminalStage 以稳定 terminalId 承载终端，布局树只决定几何位置；标签切换、分屏、交换和迁移不重建终端。存档只保存布局和连接引用，打开创建新会话。详见 [工作台与工作区](18-terminal-workspaces.md)。

## 4. 数据、设置与权限

保险库解锁后才允许业务操作。storage 完成迁移及外键检查后发布数据库池；主密钥模块协调认证元数据、解锁和密钥切换。改密在加密副本上执行，通过数据库生命周期门闩和可切换连接池协调发布，不重建活动终端。已应用迁移保持原始字节，格式与恢复规则见 [存储](07-storage.md)。

主机清单通过 `list_host_inventory` 读取同一事务中的主机、分组、标签和认证摘要；清单、单项读取和普通保存返回共用同一投影，写后投影在提交前完成；单项读取不组装整份清单。前端读写代次隔离迟到响应，Rust revision 与事务保证并发写入边界。

设置为独立单实例窗口。Rust 串行合并局部设置并发出失效通知，各窗口重读后热应用，旧草稿不能覆盖即时偏好。settings 窗口仅获设置、专用改密、JSON 导出与备份权限，通用会话 owner 仍仅 main；命令 ACL、本地来源和资源归属共同授权，见 [安全](08-security.md)。

备份分为 storage 的白名单快照/新增合并、security 的压缩/加密、壳的 provider/编排三层；provider 仅处理不透明包，增加云厂商不改变终端核心。详见 [备份](22-data-backup.md)。

## 5. 文件与转发边界

单 SSH 终端侧栏借用该终端的 SFTP 子系统并随其关闭；固定双栏使用两个独立文件会话，转发使用独立转发会话。文件正文与转发载荷不经过 WebView，文件命令/进度不复用终端帧或 ACK。

文件前端共享目录控制，侧栏与双栏保留各自连接切换策略。壳的 `files/transfer.rs` 统一任务登记和作用域清理，commands 保留 owner、操作锁、许可及 Channel。上传与对拷共享 SFTP 目标写入边界；本地授权/源复核与远端扫描仍按端点语义分别处理。

原生文件授权和平台拖放在壳内，应用内布局/对拷在前端。通用连接租约、本地文件 provider 和完整 TransferManager 尚未实现；现有登记复用不等于统一扫描/执行层。行为与路径安全见 [SFTP](16-sftp-workspace.md)，监听与流生命周期见 [转发](21-port-forwarding.md)。

## 6. 关键取舍

- 桌面本地采用 Tauri + Rust + Vue，协议核心独立，不引入 HTTP/WebSocket 后端。
- 消费后 ACK 与有界队列允许反压远端输出；取消独立运行，稳定终端需要旧流排空和代次隔离。
- 全库加密，OS 仅缓存主密钥；数据库与元数据配对，改密使用额外磁盘上的加密副本。
- 文件/转发专用连接避免依赖终端 Shell，代价是尚无通用连接复用租约。
- 白名单逻辑备份排除本机设置和日志，恢复使用新 ID 新增合并。
- Naive UI 采用薄适配，应用主色与终端配色独立；只按实际职责收敛重复，不按行数机械拆分或为未来能力预建抽象。

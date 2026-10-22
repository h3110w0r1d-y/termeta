# 代码结构评估与渐进重构方案

更新：2026-10-22。本文维护职责归属与剩余结构问题；当前分层见 [系统架构](03-architecture.md)，功能状态、运行验收和交付顺序分别见 [产品需求](01-product-requirements.md)、[路线](10-roadmap.md)。源码存在不代表编译或运行验证通过。

## 1. 组织原则与当前职责

Tauri 2 + Rust + Vue 3/TS 分层适合现有功能。结构优化按资源生命周期和事务职责进行，优先提取重复控制逻辑，不按文件行数拆透传组件，也不为未实现能力建立空目录、通用服务或新 crate。

| 功能职责 | 当前归属 | 边界 |
| --- | --- | --- |
| 窗口与连接入口 | `src/layout/` | 装配门禁、订阅、Header、导航外观和窗口操作，不持有终端正文 |
| 工作台 | `src/modules/workbench/` | AppTabs 协调标签，WorkbenchTab 呈现单项；布局与焦点独立于窗口装配 |
| 终端运行期 | `src/modules/terminal/registry.ts`、`runtime/` | 稳定实例、流和统一关闭；成功归档后才移除视图 |
| 通用控制 | `src/modules/sessions/` | 三种用途的轻量投影、挑战和心跳，不持连接资源 |
| 主题 | `src/theme/` | 共享配色模型、解析与格式映射；根应用、局部控件和实际 xterm 渲染各留在消费边界 |
| 文件交互 | `src/modules/files/` | 共享目录控制与任务摘要；侧栏和双栏各自保留连接与布局策略 |
| 桌面入口 | `src-tauri/src/commands/` | DTO、调用窗口、owner、Channel 与原生适配 |
| 文件服务 | `src-tauri/src/files/` | 浏览器获取、上传下载及传输登记清理；正文留在 Rust |
| 协议、数据与事务 | `crates/` | core 保持协议无关；协议适配、存储和保险库按既有端口协作 |

页面拥有展示，控制器拥有操作期，Store 保存轻量投影。目录导航、输入顺序和订阅重试等共享控制逻辑应集中；不把终端条目变成通用文件任务容器，也不以复制响应式字段来拆分 NForm。简单转发命令无需再包一层无行为 Service。

已有一致快照、独立输入泵、原子凭据写入、统一关闭和附属资源撤销的契约分别由 [存储](07-storage.md)、[会话模型](04-session-model.md)、[IPC](06-ipc-and-streaming.md) 与 [日志](23-snippets-and-session-logs.md) 维护，不在本文重复历史修复清单。

## 2. 文件工作区：端点、任务与执行

目录分页、选择及迟到读取隔离由 [useSftpDirectory](../src/modules/files/useSftpDirectory.ts) 共享；侧栏保留断线前列表，独立端点在切换连接时清空，不能因共用控制器而混同这两种交互。侧栏 [任务队列](../src/modules/files/useSftpTransferQueue.ts) 使用判别联合，传输状态通过窄回调投影，不依赖 TerminalItem。双栏 [SftpBrowser](../src/modules/files/SftpBrowser.vue) 仍独立持有对拷取消与进度。

壳的 [TransferRegistration](../src-tauri/src/files/transfer.rs) 统一普通传输、对拷及原生拖出的登记与 RAII 清理；commands 仍负责 owner、操作锁、全局许可和 Channel，不表示已具备完整任务执行器。

剩余结构问题是 [FileProvider](../crates/terminal-files/src/lib.rs) 仅提供目录读取，[SFTP copy](../crates/terminal-sftp/src/copy.rs) 的两端固定为 SftpBrowser；本地上传、下载及远端对拷的扫描和执行分散。增加本地文件栏前，应按真实复制方向逐步完善下列职责：

| 层 | 演进职责 | 保留边界 |
| --- | --- | --- |
| Vue 文件界面 | 端点引用、共享任务摘要及导航/选择投影 | 侧栏和双栏布局分开；原生拖出兑现前不显示任务 |
| 壳内文件服务 | 本地授权、操作占用、任务许可、取消和确认适配 | commands 保留授权入口，不接受任意本地路径 |
| terminal-files | 按需求补 metadata、目录及 reader/writer 端口，共享扫描计划和流式执行 | 复用现有有界复制，不依赖 Tauri |
| local/SFTP 适配 | 平台访问、协议请求、句柄关闭及各自路径语义 | 本地路径不使用远端字符串拼接规则 |

统一任务摘要后再收敛关闭确认，避免长期同时依赖全局和每终端的传输布尔投影。原生覆盖确认用窄回调注入；先一个实际复制方向，再扩展共享执行，不一次构造覆盖所有未来功能的大接口。

连接租约先解决独立连接的持有与释放：明确 owner、用途、配置快照、身份、最后持有者及断线传播后，再考虑 SSH 池化。侧栏仍随终端关闭，固定双栏仍为独立连接。通用 TransferManager、本地文件端点、持久化队列、断点续传及跨重启恢复尚未实现；完整文件工作区不作为验收现有 SFTP 的前置。

行为和安全边界以 [SFTP](16-sftp-workspace.md) 为准。后续执行层调整须保留反向对拷不死锁、许可释放、进度通道失效、取消早于登记及部分完成结果；上传合并不承诺回滚，下载拒绝覆盖。

## 3. 区分 wire DTO、领域配置与展示摘要

[TS HostProfile](../src/services/desktop/types.ts) 继承 SSH 配置，Serial/Telnet/Workspace 被迫携带无关 SSH 字段，kind 不能保证可选配置存在；Rust [model](../crates/terminal-storage/src/model.rs) 同样使用扁平结构和自由字符串 kind，[工作区存储](../crates/terminal-storage/src/workspaces.rs) 因而要构造并校验空 SSH 字段。混合展示连接和工作区只需要公共元数据，无需共用完整 SSH 领域形状。

保留现有数据库及 IPC 兼容表示，先在单一适配层映射为“共享保存项元数据 + SSH/Telnet/Serial/Workspace 判别联合”。编辑器使用对应草稿，连接展开仅接受真实连接，工作区布局在类型上必须存在。分组、排序等公共操作逐步使用窄命令或明确映射，不反复提交完整旧配置。已有 OpenTarget 判别联合可作为方向。

TS 业务模型归对应领域，wire 契约留 desktop 层；Rust 在序列化和存储读取边界显式映射，并提供旧配置、损坏字段、工作区引用和备份往返兼容样例。变更 wire 时同步 Rust/TS 与版本处理；Local cwd/env/argv、Serial HEX 不继续堆入 SSH 默认字段。

设置的兼容规则也分散于 [Rust 校验](../src-tauri/src/settings.rs)、[局部保存](../src-tauri/src/commands/settings.rs) 和 [TS 归一化](../src/modules/settings/store.ts)。保留串行局部保存与多窗口热应用，将 normalize/validate/patch 规则集中为可复用入口，逐步采用类型化 patch 或共享契约样例；不改回整份草稿覆盖，也不将前端主题展示数据整体搬进 Rust。

## 4. 契约与验证边界

[结构守卫](../scripts/check-structure.mjs) 检查命令清单、handler、字面量 invoke 和 capability 入口，自定义权限组按显式 `commands.allow` 解析；它不生成类型，也不证明序列化一致、最小权限或运行时 owner 校验正确。`Channel<T>` / `invoke<T>` 泛型不提供运行时验证。

后续按领域补 Rust/TS 序列化样例及兼容输入，继续保持唯一 desktop 门面。是否生成 TS 取决于维护成本，并须核对具体依赖 release/API/MSRV/许可，不为生成器预先新增依赖。

回归源码与覆盖范围由 [测试说明](../tests/README.md) 维护；真实 SSH/PTY/Serial、SQLCipher、云备份、原生对话框及三平台退出/压力的门槛统一见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。归档等并发预算仍是保守边界，须实测后调整。所有编译、测试、启动及连接遵循 [协作规则](../AGENTS.md)，静态检查不能替代运行证据。

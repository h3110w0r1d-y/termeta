# 代码结构评估与渐进重构方案

更新：2026-10-27。本文只维护剩余结构问题与演进边界；当前分层见 [系统架构](03-architecture.md)，功能状态、运行验收和交付顺序分别见 [产品需求](01-product-requirements.md)、[路线](10-roadmap.md)。源码存在不代表编译或运行验证通过。

## 1. 组织原则

Tauri 2 + Rust + Vue 3/TS 分层适合现有功能。结构优化按资源生命周期和事务职责进行，优先提取重复控制逻辑，不按文件行数拆透传组件，也不为未实现能力建立空目录、通用服务或新 crate。

页面拥有展示，控制器拥有操作期，Store 保存轻量投影。目录导航、输入顺序和订阅重试等共享控制逻辑应集中；不把终端条目变成通用文件任务容器，也不以复制响应式字段来拆分 NForm。简单转发命令无需再包一层无行为 Service。

## 2. 文件工作区：端点、任务与执行

目录分页、选择及迟到读取隔离由 [useSftpDirectory](../src/modules/files/useSftpDirectory.ts) 共享；侧栏保留断线前列表，独立端点在切换连接时清空，不能因共用控制器而混同这两种交互。侧栏 [任务队列](../src/modules/files/useSftpTransferQueue.ts) 使用判别联合，传输状态通过窄回调投影，不依赖 TerminalItem。双栏 [SftpBrowser](../src/modules/files/SftpBrowser.vue) 仍独立持有对拷取消与进度。

壳的 [TransferRegistration](../src-tauri/src/files/transfer.rs) 统一普通传输、对拷及原生拖出的登记与 RAII 清理；commands 仍负责 owner、操作锁、全局许可和 Channel，不表示已具备完整任务执行器。

上传与对拷的目标元数据、真实目录合并、覆盖决策和写句柄集中于 [SFTP 写入边界](../crates/terminal-sftp/src/upload.rs)，原生确认通过窄回调留在壳。源扫描与打开复核仍按本地授权和远端协议分别处理，不能为共用目标而混同源端路径与取消语义。

剩余结构问题是 [FileProvider](../crates/terminal-files/src/lib.rs) 仅提供目录读取，[SFTP copy](../crates/terminal-sftp/src/copy.rs) 的两端固定为 SftpBrowser；本地上传、下载及远端对拷的扫描和执行分散。增加本地文件栏前，应按真实复制方向逐步完善下列职责：

| 层 | 演进职责 | 保留边界 |
| --- | --- | --- |
| Vue 文件界面 | 端点引用、共享任务摘要及导航/选择投影 | 侧栏和双栏布局分开；原生拖出兑现前不显示任务 |
| 壳内文件服务 | 本地授权、操作占用、任务许可、取消和确认适配 | commands 保留授权入口，不接受任意本地路径 |
| terminal-files | 补 metadata、目录操作端口，复用现有 FileReader/FileWriter 与有界复制，收敛扫描和执行 | 不依赖 Tauri |
| local/SFTP 适配 | 平台访问、协议请求、句柄关闭及各自路径语义 | 本地路径不使用远端字符串拼接规则 |

统一任务摘要后再收敛关闭确认，避免长期同时依赖全局和每终端的传输布尔投影。原生覆盖确认用窄回调注入；先一个实际复制方向，再扩展共享执行，不一次构造覆盖所有未来功能的大接口。

连接租约先解决独立连接的持有与释放：明确 owner、用途、配置快照、身份、最后持有者及断线传播后，再考虑 SSH 池化。侧栏仍随终端关闭，固定双栏仍为独立连接。通用 TransferManager、本地文件端点、持久化队列、断点续传及跨重启恢复尚未实现；完整文件工作区不作为验收现有 SFTP 的前置。

行为和安全边界以 [SFTP](16-sftp-workspace.md) 为准。后续执行层调整须保留反向对拷不死锁、许可释放、进度通道失效、取消早于登记及部分完成结果；上传合并不承诺回滚，下载拒绝覆盖。

## 3. 区分 wire DTO、领域配置与展示摘要

[TS 保存项模型](../src/modules/hosts/model.ts) 使用共享元数据与 SSH/Telnet/Serial/Workspace 判别联合，串口配置和工作区布局在对应类型中必填；连接展开排除工作区，网络与串口编辑器只投影对应草稿。排序、名称等公共操作使用窄元数据形状。

数据库与 [wire DTO](../src/services/desktop/types.ts) 保留原兼容表示，[desktop 适配](../src/services/desktop/hosts.ts) 集中双向映射、必需配置缺失与跨协议字段拒绝；空 SSH 占位字段只在兼容出口构造。映射不替代完整运行时 schema 校验，也不跳过损坏清单项。

剩余问题在 Rust [model](../crates/terminal-storage/src/model.rs) 的扁平结构和自由字符串 kind，[工作区存储](../crates/terminal-storage/src/workspaces.rs) 仍需构造空 SSH 字段。后续在序列化和存储边界显式映射，保留现有 schema、备份与 IPC 表示，补业务数据往返兼容证据。变更 wire 时同步 Rust/TS 与版本处理；Local cwd/env/argv、Serial HEX 不继续堆入 SSH 默认字段。

设置的 Rust 归一化、校验和 patch 合并集中于 [设置规则](../src-tauri/src/settings.rs)，[命令](../src-tauri/src/commands/settings.rs) 负责授权、版本、串行读写和通知；[TS 状态规则](../src/modules/settings/state.ts) 负责默认值和 UI 归一，[Store](../src/modules/settings/store.ts) 只管理状态与局部保存队列。两端规则分别维护，变更时须同步核对旧设置、嵌套合并与按 ID 删除的契约，并通过授权运行验收确认。保持多窗口热应用，不恢复整份草稿覆盖。

## 4. 契约与验证边界

[结构守卫](../scripts/check-structure.mjs) 检查命令清单、handler、字面量 invoke 和 capability 入口，自定义权限组按显式 `commands.allow` 解析；它不生成类型，也不证明序列化一致、最小权限或运行时 owner 校验正确。`Channel<T>` / `invoke<T>` 泛型不提供运行时验证。

Rust/TS 序列化表示和兼容输入须按领域同步维护，继续保持唯一 desktop 门面。是否生成 TS 取决于维护成本，并须核对具体依赖 release/API/MSRV/许可，不为生成器预先新增依赖。

当前仓库不保留自动测试源码；保留的静态检查入口见 [验证说明](../tests/README.md)。真实 SSH/PTY/Serial、SQLCipher、云备份、原生对话框及三平台退出/压力的门槛统一见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。归档等并发预算仍是保守边界，须实测后调整。所有编译、启动及连接遵循 [协作规则](../AGENTS.md)，静态检查不能替代运行证据。

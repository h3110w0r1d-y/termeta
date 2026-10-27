# Termeta 文档导航

更新：2026-10-27。文档描述当前代码的行为与有效约束，不保留逐轮实现日志。源码完成与运行验收分开记录，当前版本尚未完成完整集成、三平台和发布验收。

## 阅读入口

开发前阅读 [协作规则](../AGENTS.md)、[产品需求](01-product-requirements.md)、目标领域文档及 [路线](10-roadmap.md)。功能完成状态只维护在产品需求，交付顺序只维护在路线；具体字段、常量和组件流程以源码为准。

| 文档 | 维护内容 |
| --- | --- |
| [产品需求与状态](01-product-requirements.md) | 功能编号/完成标记、源码入口、已知风险与验收边界 |
| [依赖说明](02-library-research.md) | 锁定依赖、官方依据、许可与升级注意事项 |
| [系统架构](03-architecture.md) | 分层、依赖方向与关键设计取舍 |
| [会话模型](04-session-model.md) | actor、状态、生命周期与 Local 边界 |
| [IPC 与数据流](06-ipc-and-streaming.md) | 授权入口、二进制帧、ACK、输入顺序和尾输出 |
| [存储与恢复](07-storage.md) | 加密格式、schema、迁移、凭据事务与故障恢复 |
| [安全](08-security.md) | 威胁模型、权限、秘密与不可信输入 |
| [桌面交互与设置](09-desktop-ux.md) | 页面、窗口、语言、外观/配色和前端边界 |
| [路线](10-roadmap.md) | 阶段、依赖、剩余门槛和执行顺序 |
| [快捷键](14-shortcuts.md) | 默认绑定、输入路由、冲突和原生菜单例外 |
| [工作台与工作区](18-terminal-workspaces.md) | 布局、稳定终端、广播与显式存档 |
| [SSH](05-ssh-design.md) | 信任、认证、Shell/exec 与一次性系统识别 |
| [SFTP](16-sftp-workspace.md) | 浏览/传输、覆盖语义、原生拖放与资源限额 |
| [Serial](19-serial-terminal.md) | 设备独占、参数、字符集与控制线边界 |
| [Telnet](20-telnet-terminal.md) | 协商、NVT/BINARY 与明文登录边界 |
| [端口转发](21-port-forwarding.md) | 三模式、独立连接、监听与取消 |
| [Snippets 与会话日志](23-snippets-and-session-logs.md) | 片段管理、关闭归档、退出握手与日志回看 |
| [备份与恢复](22-data-backup.md) | 第三方 provider、独立密码、版本选择与新增合并 |
| [代码结构评估与渐进重构](27-architecture-refactoring.md) | 剩余结构问题与渐进拆分边界 |

文件编号保留空缺，不为连续编号重命名。

## 术语与资源

- profile/保存项是持久化配置；session 是一次后端连接；terminalId 是跨重连稳定的视图标识。
- Workbench 是整个工作台，TerminalTabGroup 是运行标签组，Workspace 是多终端工作区及其显式存档。
- IPC 命令使用 snake_case，JSON 字段使用 camelCase；接口以 Rust/TS 同步类型为准。
- 字体、许可证与摘要见 [字体清单](../resources/fonts/README.md)；其他素材来源见 [资源说明](../resources/README.md)；静态检查与运行验收边界见 [验证说明](../tests/README.md)，当前仓库不保留自动测试源码。

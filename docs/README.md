# Termeta 开发文档

更新：2026-10-17。M1/M2 SSH 主体源码已落地，尚未完成 MVP 验收。文档保留最终状态与有效设计；当前范围统一见 [需求与实现状态](01-product-requirements.md#8-当前实现与限制)，待办见 [路线](10-roadmap.md)，验收见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。

## 当前基线

Windows/macOS/Linux 桌面客户端，Tauri 2 + Rust + Vue 3 + TypeScript，通用 UI 为 Naive UI 2.45.3。源码覆盖 SSH/Telnet/基础 Local 与 Serial、主机/分组/凭据、稳定终端、分屏广播、显式工作区保存、独立设置、SFTP、三模式端口转发及 Gist/WebDAV 手动加密备份与非覆盖同步。

断线保留 terminalId/xterm/历史，重连替换后端连接；工作区存档只保存布局和连接引用，打开创建新会话，不自动恢复标签。SQLCipher 加密格式2、schema 6，同格式 schema 2–5 升级，OS 仅缓存主密钥，直接保存数据库文件时须与 vault 元数据配对。云备份使用独立密码封装白名单 SQLite 快照，排除本机设置；支持首次初始化云拉取并保存本机云端配置。限制与完整功能表统一维护于产品需求，不在导航重复展开。

## 导航

| 编号 | 文档                                      | 主要内容                                         |
| ---- | ----------------------------------------- | ------------------------------------------------ |
| 1    | [产品需求](01-product-requirements.md)    | 需求完成勾选、源码入口、限制、公告与验收边界     |
| 2    | [依赖预研](02-library-research.md)        | 官方资料核查、适配结论、风险和来源               |
| 3    | [系统架构](03-architecture.md)            | 分层、目录、依赖与扩展接口                       |
| 4    | [会话模型](04-session-model.md)           | 生命周期、actor、协议无关接口                    |
| 5    | [SSH 设计](05-ssh-design.md)              | 信任与认证、Shell、超时与重连                    |
| 6    | [IPC 与流式数据](06-ipc-and-streaming.md) | 命令契约、字节帧、ACK、背压                      |
| 7    | [存储设计](07-storage.md)                 | schema、迁移、数据边界                           |
| 8    | [安全设计](08-security.md)                | 凭据、Tauri 权限、终端输出与日志                 |
| 9    | [桌面交互](09-desktop-ux.md)              | 页面、标签、快捷键、可访问性                     |
| 10   | [开发路线](10-roadmap.md)                 | 分阶段交付、任务依赖、当前进度                   |
| 12   | [架构决策](12-decisions.md)               | 有效决策、理由与变更条件                         |
| 13   | [UI 选型](13-ui-stack.md)                 | Naive UI 适配性、控件分工、主题与桌面集成        |
| 14   | [快捷键设置](14-shortcuts.md)             | 应用内全局/终端快捷键、鼠标组合与冲突路由        |
| 15   | [终端配色](15-terminal-themes.md)         | 内置方案、完全自定义取色、导入导出与热更新       |
| 16   | [SFTP 工作区](16-sftp-workspace.md)       | 目录浏览、上传下载、原生拖放与资源边界           |
| 18   | [终端工作区](18-terminal-workspaces.md)   | 标签排序、四向拖入、嵌套分屏、输入广播与终端保活 |
| 19   | [Serial终端](19-serial-terminal.md)       | 串口枚举、启动参数、字符集与设备生命周期         |
| 20   | [Telnet终端](20-telnet-terminal.md)       | Telnet协商、主机配置、明文边界与schema升级       |
| 21   | [端口转发](21-port-forwarding.md)         | 本地/远程/SOCKS5、SSH引用、生命周期与监听边界    |
| 22   | [备份与同步](22-data-backup.md)           | Gist/WebDAV、加密快照、非覆盖合并与首次云端初始化 |

协作与执行规则见 [AGENTS.md](../AGENTS.md)。未经用户明确要求，不构建、启动、执行 Rust 测试或连接真实主机；允许不触发构建的依赖维护与静态检查。静态检查通过不代表运行验收；用户既有 macOS 反馈不外推为当前版本或三平台验收。

本机Telnet手动测试容器配置与连接参数见[测试环境说明](../dev/telnet/README.md)，协议及验证范围见[Telnet设计](20-telnet-terminal.md)。容器就绪不代表应用已验收。

## 阅读约定

- 产品需求表 `[x]` 仅表示代码已完整实现，不考虑测试；`[ ]` 包含未实现和部分实现，剩余范围就近说明。源码完成与运行验收分开记录。
- 导航编号沿用文件名前缀，删除文档后保留编号空缺；实现状态和简要验收边界统一维护于产品需求。
- M0预研，M1 SSH纵向切片，M2 SSH MVP，M3专业SSH/SFTP，M4 Local/Serial/Telnet，M5分发产品化；P0/P1/P2按所属里程碑理解。
- host/profile为持久化连接配置，session为一次后端连接生命周期，connection为底层SSH传输，terminalId为跨重连稳定的终端视图标识，pane/tab为布局节点。
- IPC命令使用snake_case，JSON字段使用camelCase；SSH/Local字节流保留原始字节，Telnet剥离协议协商并按NVT/BINARY处理控制字节，Serial文本模式可按字符集转码，详见串口设计。具体接口以同步的Rust/TS类型为准。
- 字体资源及许可证见 [字体清单](../public/fonts/README.md)，配色、拼音表及SSH系统图标/只读命令来源见 [静态资源](../resources/README.md)。

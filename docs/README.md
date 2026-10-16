# Termvexa 开发文档

更新：2026-10-16。M1/M2 SSH 主体源码已落地，尚未完成 MVP 验收。文档保留最终状态与有效设计；当前范围统一见 [需求与实现状态](01-product-requirements.md#8-当前实现与限制)，待办见 [路线](10-roadmap.md)，验收见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。

## 当前基线

Windows、macOS、Linux 桌面客户端，采用 Tauri 2 + Rust + Vue 3 + TypeScript，通用控件为 Naive UI 2.45.3，工作区使用自定义标签。已实现SSH与基础本地终端，Serial、隧道/跳板/Agent 为后续范围。

已落源码包括左侧分组树/主机计数/展开与包含子组持久化、右侧按组浏览/收藏置顶、顶部独立拼音搜索下拉、分组右键管理/两步删除、主机和分组拖拽迁移、凭据管理、SSH/动态认证/主机信任、终端保活与重连、标签排序和嵌套分屏、输入广播、快捷键、12种本地字体和64套终端配色。SFTP 提供单终端浏览/上传下载侧栏、目录合并上传及 macOS 文件/目录承诺拖出；固定双栏支持保存主机的独立 SSH/SFTP 连接与 Web 栏间文件/目录对拷，Windows/Linux 原生拖出未实现。

断线与失败保留原 Tab、terminalId 和 xterm 缓冲，重连只替换后端连接。主机和凭据由Rust写入SQLCipher加密库 `termvexa-encrypted-v2.sqlite3`（schema 2），首次密码经Argon2id派生主密钥；OS keyring只缓存主密钥，失败时密码恢复。配套vault.json须一起保留，不迁移旧库。正文与文件数据不进入 Vue/Pinia。

## 导航

| 编号 | 文档                                        | 主要内容                                         |
| ---- | ------------------------------------------- | ------------------------------------------------ |
| 1    | [产品需求](01-product-requirements.md)      | 需求完成勾选、源码入口、限制、公告与验收边界                   |
| 2    | [依赖预研](02-library-research.md)          | 官方资料核查、适配结论、风险和来源               |
| 3    | [系统架构](03-architecture.md)              | 分层、目录、依赖与扩展接口                       |
| 4    | [会话模型](04-session-model.md)             | 生命周期、actor、协议无关接口                    |
| 5    | [SSH 设计](05-ssh-design.md)                | 信任与认证、Shell、超时与重连                    |
| 6    | [IPC 与流式数据](06-ipc-and-streaming.md)   | 命令契约、字节帧、ACK、背压                      |
| 7    | [存储设计](07-storage.md)                   | schema、迁移、数据边界                           |
| 8    | [安全设计](08-security.md)                  | 凭据、Tauri 权限、终端输出与日志                 |
| 9    | [桌面交互](09-desktop-ux.md)                | 页面、标签、快捷键、可访问性                     |
| 10   | [开发路线](10-roadmap.md)                   | 分阶段交付、任务依赖、当前进度                   |
| 12   | [架构决策](12-decisions.md)                 | 有效决策、理由与变更条件                         |
| 13   | [UI 选型](13-ui-stack.md)                   | Naive UI 适配性、控件分工、主题与桌面集成        |
| 14   | [快捷键设置](14-shortcuts.md)               | 应用内全局/终端快捷键、鼠标组合与冲突路由        |
| 15   | [终端配色](15-terminal-themes.md)           | 内置方案、完全自定义取色、导入导出与热更新       |
| 16   | [SFTP 工作区](16-sftp-workspace.md)         | 目录浏览、上传下载、原生拖放与资源边界           |
| 18   | [终端工作区](18-terminal-workspaces.md)     | 标签排序、四向拖入、嵌套分屏、输入广播与终端保活 |

协作与执行规则见 [AGENTS.md](../AGENTS.md)。未经用户明确要求，不构建、启动、执行 Rust 测试或连接真实主机；允许不触发构建的依赖维护与静态检查。静态检查通过不代表运行验收；用户既有 macOS 反馈不外推为当前版本或三平台验收。

## 阅读约定

- 产品需求表 `[x]` 仅表示代码已完整实现，不考虑测试；`[ ]` 包含未实现和部分实现，剩余范围就近说明。源码完成与运行验收分开记录。
- 导航编号沿用文件名前缀，删除文档后保留编号空缺；实现状态和简要验收边界统一维护于产品需求。
- M0预研，M1 SSH纵向切片，M2 SSH MVP，M3专业SSH/SFTP，M4 Local/Serial，M5分发产品化；P0/P1/P2按所属里程碑理解。
- host/profile为持久化连接配置，session为一次后端连接生命周期，connection为底层SSH传输，terminalId为跨重连稳定的终端视图标识，pane/tab为布局节点。
- IPC命令使用snake_case，JSON字段使用camelCase，字节流保留原始字节。具体接口以同步的Rust/TS类型为准。
- 字体资源及许可证见 [字体清单](../public/fonts/README.md)，配色与拼音表来源见 [静态资源](../resources/README.md)。

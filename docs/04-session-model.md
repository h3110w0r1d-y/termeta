# 统一会话模型与生命周期

更新：2026-10-18。本文维护协议无关的身份、状态与资源契约。功能及验收状态见 [产品需求](01-product-requirements.md#8-当前实现与限制)，字节流协议见 [IPC](06-ipc-and-streaming.md)。

## 1. 身份与所有权

| 标识 | 生命周期与用途 |
| --- | --- |
| ProfileId | 保存的连接配置；建立会话时由 Rust 读取，后续编辑不改变已运行的配置快照 |
| TerminalId | 终端视图身份；跨失败、断线和重连稳定，用户关闭视图才释放 xterm 与滚屏缓冲 |
| SessionId | 后端一次连接尝试；重连创建新值，终态不结束终端视图生命周期 |
| RequestId | 同一 owner 的打开幂等标识；重复请求返回原快照，不替换 Channel 或重复连接 |
| ChallengeId | 一次信任或认证交互；绑定会话、阶段、owner 和期限，只能消费一次 |
| Owner | Tauri 注入的窗口身份；不采信前端提供的身份字符串 |

当前没有公开 ConnectionId 或跨功能共享连接租约。终端、独立文件、转发分别使用 `sessions`、`file_sessions`、`forward_sessions`。

## 2. 状态机

SSH 正常路径为 `Created → Connecting → Authenticating → Opening → Connected`，未知主机密钥和认证输入分别进入 `AwaitingHostKey`、`AwaitingAuth`。Local、Serial 和 Telnet 按各自驱动跳过不适用阶段；Telnet 的 Connected 仅表示 TCP 已建立，登录提示是普通终端输出。

- 正常远端退出：`Closing → Closed`，保留退出码或信号。
- 建立阶段错误：Failed；已连接后的协议或网络错误：Disconnected。
- 显式取消可中止任意非终态，撤销挑战并进入关闭流程。

Closed、Failed、Disconnected 都要求释放协议资源，不是保留后台连接的暂停态。三种终态均保留前端视图、历史和错误信息；后端摘要过期也不能清除终端历史。

## 3. 核心接口与任务

[SessionFactory](../crates/terminal-core/src/ports.rs) 提供 `kind`、`capabilities`、`validate`、`run`；[SessionManager](../crates/terminal-core/src/manager.rs) 注册工厂并负责打开、查询、write、resize、ACK、关闭、幂等和配额，不暴露协议对象。

每个会话由驱动持有连接或设备，`DriverContext` 提供独立取消信号、输入接收器、最新尺寸、输出发送器、状态事件和挑战 broker。输入有界且按序单写；resize 合并为最新值；取消与挑战响应不进入可能被输入堵塞的队列。信任与凭据端口由具体工厂持有，core 不访问数据库或钥匙串。

输出 forwarder 独立等待消费信用；控制事件不依赖输出 ACK。驱动暂停读取时仍须响应取消，耗时写入可取消，交付不确定时不得自动重发。SSH 库内部窗口和队列的公平性仍需运行验收。

每个 manager 最多 20 个活动会话、4 个建立并发；独立文件入口另限两条连接。活动 RequestId 保留至终态，终态摘要最多保留 10 分钟、1024 条，按需淘汰；摘要不保存终端输出或完整配置。

## 4. 打开、关闭与重连

`open_session` 校验 DTO、owner 和配额后返回 Created 快照，连接异步进行。事件可能早于命令返回；调用方预先绑定 Channel，并按 RequestId/SessionId 归并。非法配置等同步错误不创建会话。

关闭幂等且立即拒绝新输入、设置取消信号、撤销挑战。`close_session` 返回当前快照不代表清理已结束，应以终态事件或查询为准。适配器负责释放 socket、channel、设备及 worker；输出信用等待不能阻塞关闭。正常结束最多等待 3 秒排空输出，显式取消允许丢弃尾部；先汇合 forwarder 冻结偏移，再发布终态，详见 [IPC](06-ipc-and-streaming.md)。

“断开”只关闭连接；“关闭标签/窗格”才额外释放视图。重连使用新的 RequestId、SessionId 和 Channel，保留 TerminalId、xterm 与历史；清空旧输入队列，不重放输入。旧流先完成结束协调，新旧回调按 SessionId 和绑定代次隔离；重复重连须合并为一次操作。

窗口销毁或心跳失联关闭所属会话，切页和隐藏标签不关闭。进程崩溃依靠 OS 释放网络资源；专门睡眠恢复和完整本地进程树回收尚未实现。

## 5. 驱动能力与边界

| 驱动 | 输出 | resize / PTY | 特有边界 |
| --- | --- | --- | --- |
| [SSH](05-ssh-design.md) | 原始字节 | 是 / 是 | 主机信任、密码/公钥及动态交互认证；Shell 或 PTY exec |
| Local | 原始字节 | 是 / 是 | 当前系统 Shell、主目录、继承环境；自定义 Shell/cwd/env 待实现 |
| [Serial](19-serial-terminal.md) | 流式转码为 UTF-8 | 否 / 否 | 设备独占；无会话内控制线命令或原始 HEX 通道 |
| [Telnet](20-telnet-terminal.md) | 剥离 IAC、处理 NVT/BINARY 后的字节 | NAWS / 否 | 明文终端登录；NAWS 须服务端接受 |

Local 使用阻塞读写线程和有界桥，输出排空后发布终态；取消清理子进程，晚到的 PTY 创建结果也由 RAII 回收。Unix 结束初始进程组；脱离进程组的子进程及 Windows 完整进程树仍待完善。实现见 [terminal-local](../crates/terminal-local/src/lib.rs)。

独立文件和转发复用状态、挑战与取消，不申请 PTY/Shell，也不记录终端会话日志；分别见 [SFTP](16-sftp-workspace.md)、[端口转发](21-port-forwarding.md)。

## 6. 工作台隔离与错误

标签组、TerminalId、SessionId 相互独立；排序、分屏、区域交换与迁移只改变布局，不重建会话。输入广播逐目标提交，每个目标保持独立序号、字节预算和连接代次，断线不补发。保存工作区只保存布局及配置引用，重新打开创建新会话，详见 [工作区](18-terminal-workspaces.md)。

`SessionError` 包含 `code`、`phase`、`retryable`、`message`，关联身份由事件或命令信封携带；不直接序列化原始错误。retryable 表示允许重新尝试连接，不授权重放 write。

源码入口：[types](../crates/terminal-core/src/types.rs)、[manager](../crates/terminal-core/src/manager.rs)、[session](../crates/terminal-core/src/session.rs)、[stream](../crates/terminal-core/src/stream.rs)。core/Serial 的历史局部测试不覆盖当前全部集成；当前验收边界统一见 [产品需求](01-product-requirements.md#11-运行验收边界)。

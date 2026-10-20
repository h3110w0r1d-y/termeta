# IPC 契约与终端数据流

更新：2026-10-19。当前终端协议版本为 1。本文维护跨语言和流控约束，具体 DTO 与命令签名以源码为准，不复制完整类型定义。

## 1. 契约入口与授权

| 入口 | 内容 |
| --- | --- |
| [TS 门面](../src/services/desktop/client.ts)、[TS 类型](../src/services/desktop/types.ts) | 前端唯一 IPC 入口与参数/结果类型 |
| [Rust 命令](../src-tauri/src/commands/mod.rs)、[核心类型](../crates/terminal-core/src/types.rs) | DTO、运行时校验、状态与事件 |
| [命令名单](../src-tauri/src/command_names.rs)、[构建清单](../src-tauri/build.rs) | AppManifest 自定义命令 ACL |
| [资源所有权](../src-tauri/src/state.rs)、`src-tauri/capabilities/` | 本地来源、调用窗口、解锁状态和资源归属 |

命令采用 snake_case，JSON 字段采用 camelCase。业务 ID、版本、状态、长度和枚举均由 Rust 校验；前端类型不构成授权。新增命令必须同步名单、handler、权限和 desktop 门面。

业务命令通常只接受已解锁 main；设置及维护命令使用受限的 main/settings 授权。账号命令允许本地main在setup/locked开始浏览器授权，settings仍需解锁；官方云密码完成仅main，已有锁定库先解锁本机再验证云端。第三方备份仅已解锁离线模式设置入口，首次第三方初始化命令已移除。锁定态原生菜单语言更新仍是专用入口，不能访问已有锁定库。完整权限见 [安全](08-security.md)，备份命令见 [备份](22-data-backup.md)，官方轻量状态/门禁与同步见[官方同步](26-official-vault-sync.md)。

## 2. 会话控制

`open_session` 接收协议版本、requestId、terminalId、目标、尺寸及输出/控制 Channel，返回初始快照，连接异步进行；同 owner/requestId 幂等。保存目标只传 profileId，由 Rust 重读配置并分派 SSH/Telnet/Serial；工作区需展开保存引用，不能当作会话协议打开。临时 SSH、Telnet、Serial 使用各自类型化配置；Local 仅接受 local 目标，没有任意命令/cwd/env 入口。

| 命令组 | 语义 |
| --- | --- |
| `write_session` | 接收 inputSeq 与字节，返回 acceptedSeq，仅确认后端有序接纳 |
| `resize_session` | 接收并返回有效行列，能力不支持时不发送 |
| `ack_session_output` | 累计确认已解析输出，offset 用十进制字符串传输 |
| `respond_host_key`、`respond_auth_challenge`、`cancel_auth_challenge` | 响应当前会话的有效 challenge；认证仅接受 savePolicy=never |
| `close_session`、快照查询与心跳 | 幂等关闭、状态读取及当前 owner 会话集合对账 |

控制 Channel 使用类型化 SessionEvent，信封包含 protocolVersion、requestId、sessionId、eventSeq。事件包括 stateChanged、hostKeyChallenge、authChallenge、hostSystemChanged、notice 和 ended；stateRevision 防止迟到快照覆盖新状态。SessionSnapshot.sshCommand 表示本次实际命令模式，不返回命令正文，前端据此隔离广播和显示关闭提醒。

输出使用原始二进制 Channel（Rust `Channel<Response>` / JS `ArrayBuffer`），不走全局事件或 Base64。两个 Channel 各自有序，但没有跨 Channel 全局顺序。ended 可能先于最后数据帧到达，不能收到终态就丢弃输出 handler 或销毁终端。

## 3. 输出帧 v1

一条 Channel 消息是一帧；头部固定 24 字节、小端序，Channel 已绑定会话，无需逐帧附加长 ID。

| 偏移 | 字节数 | 字段 | 校验 |
| --- | --- | --- | --- |
| 0 | 1 | version | 1 |
| 1 | 1 | flags | 0 |
| 2 | 2 | reserved | 0 |
| 4 | 4 | frameSeq | u32，从 1 连续递增，不回绕 |
| 8 | 8 | endOffset | u64，本帧后的累计 payload 字节数 |
| 16 | 4 | payloadLength | 1–32768 |
| 20 | 4 | reserved | 0 |
| 24 | N | payload | 总长度必须等于 24 + N |

`endOffset - payloadLength` 必须等于上一帧 endOffset。JS 使用 BigInt 读取 u64，ACK 转十进制字符串；不使用 number 保存累计偏移。版本、长度、序列或偏移错误结束该流，不静默跳帧。

桥接层不做逐帧文本解码，Uint8Array 直接交给 xterm。SSH/Local 保留原始字节；Telnet 先剥离协商并处理 NVT/BINARY，Serial 文本驱动先流式转为 UTF-8。帧偏移与 ACK 始终统计进入流桥后的 payload 字节。

## 4. 背压与 ACK

```text
驱动 → 有界输出队列 → 信用控制 → raw Channel → xterm.write
                                                      ↓ 解析回调
                  释放信用 ← 累计 ACK ← 连续已解析偏移
```

| 限额 | 当前值 |
| --- | --- |
| 单帧 payload | 最大 32 KiB |
| 驱动输出队列 | 512 KiB，同时受消息数限制 |
| 未确认输出信用 | 每会话 256 KiB |
| 前端待解析队列 | 每终端流 512 KiB |
| 后端输入预算 / 单次 write | 64 KiB / 最大 16 KiB |
| ACK 合并 | 累计 64 KiB 或约 16 ms |

发送前预留信用，满足 `sentOffset - consumedOffset <= outputCredit`；Channel send 成功不表示解析完成。只能在 xterm write 回调后确认连续已解析数据。后端接受当前消费偏移的重复 ACK；新 ACK 必须前进、不超过发送偏移且落在已发送帧端点。

未确认字节耗尽信用时停止送帧，满队列反压至驱动；取消不等待信用。限额按会话生效，三类 manager 独立计数，没有统一全局字节预算。协议库、WebView、帧副本和 xterm 滚屏另占内存；限额不是实测性能或承载能力。实现见 [stream.rs](../crates/terminal-core/src/stream.rs) 与 [registry.ts](../src/modules/terminal/registry.ts)。

## 5. 输入顺序与交付边界

每会话使用 FIFO，inputSeq 从 1 连续递增，前端一次等待一个 write；onData 按 UTF-8 编码，onBinary 按单字节映射。后端拒绝跳号、内容不同的重复请求及回绕；相同序号/内容在有界缓存内返回原接纳结果，超出缓存的旧序号不重新写入。缓存最多 128 条、合计 64 KiB。

write 入队即返回 acceptedSeq，不保证远端执行完成；后续驱动写入失败通过会话终态报告错误。中断时输入可能已经部分写入，前端不自动重试，也不跨重连复用待发送输入。广播复用每个目标的 FIFO 和 write，不提供跨主机事务或同时执行保证，见 [工作区](18-terminal-workspaces.md)。

## 6. 终态、排空与旧流隔离

forwarder 停止后冻结 finalOffset，并在 ended 中返回 outputComplete、退出码/信号和错误。finalOffset 只计成功提交 Channel 的 payload；未发送尾部使 outputComplete=false。true 也不代表 JS 已解析，更不承诺尚未到达应用的远端数据。

终态后保留 3 秒 ACK 排空窗口；会话/幂等摘要最多 1024 项、最长 10 分钟。显式关闭或消费端失效可舍弃未显示尾部并标记不完整。输出信用等待超过五分钟未推进会取消。窗口销毁、owner 销毁和 Channel 发送失败负责失联清理；前端心跳仅携带三类活动会话集合，后端关闭当前 owner 中未被报告的非终态会话。

隐藏终端继续解析/ACK。重连复用原 terminalId 和 xterm，先结束旧流并等待有界解析栅栏，再绑定新的 sessionId；旧输出、状态、输入及 ACK 不得混入新流。当前不支持热重载后重新 attach 原 stream。生命周期详见 [会话模型](04-session-model.md)。

## 7. 非终端接口

主机/分组/工作区写入使用 revision 事务；秘密仅作为受控写入参数，不随读取 DTO 返回。设置采用局部合并、完整快照返回及无秘密失效通知，快捷键仅接受当前 keymap 版本 2。Snippets 使用带 revision 的管理命令；会话日志保存仅用于视图关闭时的 xterm 带样式缓冲快照（版本封装保存在 content，兼容旧纯文本），列表只返回摘要，回看按 ID 读取正文，均要求已解锁本地 main。应用退出由 Rust 拦截并通知 main，main 等待归档后通过专用完成命令放行，锁定页仅可完成退出握手，不开放日志数据权限。文件使用独立命令和进度 Channel，正文不进入 IPC；转发只发送控制状态。

模块接口边界分别见 [存储](07-storage.md)、[桌面交互](09-desktop-ux.md)、[快捷键](14-shortcuts.md)、[SFTP](16-sftp-workspace.md)、[工作区存档](18-terminal-workspaces.md)、[转发](21-port-forwarding.md)、[备份](22-data-backup.md) 和 [Snippets/日志](23-snippets-and-session-logs.md)。不要在本文重复维护全部字段或组件调用流程。

# IPC 契约与终端数据流

日期：2026-10-15。协议版本 1，首期命令/Channel/ACL 已落源码，尚未运行验证。Rust/TS 实际入口及差异见 [实施状态](17-implementation-status.md)。

## 1. 控制面与数据面

控制面采用 Tauri Commands + `Channel<SessionEvent>`，负责状态、challenge 和终态。数据面采用 `Channel<ArrayBuffer>` 接收 Rust 原始响应；输出 bytes 通过 `tauri::ipc::Response` 等锁定 release 支持的 raw Channel 方式发送。原始 ArrayBuffer 的支持依据见 [Tauri Channel API](https://v2.tauri.app/reference/javascript/api/namespacecore/)，具体实现需 SP-02 验证。

不使用全局终端 output event、不发 base64、不依赖 HTTP/WebSocket、不每个字符调用一次 IPC。两个 Channel 各自保持消息顺序，但不存在跨 Channel 的全局顺序，应用协议必须处理交错。

## 2. DTO 草案

```ts
type SessionKind = "ssh" | "local" | "serial";
type SessionState =
  | "created" | "connecting" | "awaitingHostKey"
  | "authenticating" | "awaitingAuth" | "opening"
  | "connected" | "closing" | "closed" | "failed" | "disconnected";

interface TerminalSize { cols: number; rows: number }
interface SessionSnapshot {
  protocolVersion: 1;
  requestId: string;
  sessionId: string;
  kind: SessionKind;
  title: string;
  state: SessionState;
  stateRevision: number;
  capabilities: {
    resize: boolean;
    pty: boolean;
    binaryOutput: boolean;
    serialControl: boolean;
  };
  size: TerminalSize;
}

type OpenTarget =
  | { source: "saved"; profileId: string }
  | { source: "ephemeral"; kind: "ssh"; config: SshConfigV1 };

interface SshConfigV1 {
  configVersion: 1;
  address: string;
  port: number;
  username: string;
  authPlan: AuthStepV1[];
}
type AuthStepV1 =
  | { type: "password"; credentialId?: string }
  | { type: "privateKey"; keyFileRef: string; passphraseCredentialId?: string }
  | { type: "keyboardInteractive" };
```

首期公开 OpenTarget 只接受 SSH。Local/Serial 配置在对应阶段新增受校验分支，不先允许任意对象。未传 password ref 时由认证 challenge 获取秘密，open_session 不需要明文密码字段。

前端另有不参与原生授权的稳定视图绑定：`TerminalBinding { terminalId, currentSessionId?, bindingGeneration }`。terminalId 在工作区创建时生成，重连不变；后端 sessionId 每次连接新建。前端 registry 按 terminalId 持有 xterm，session/channel 回调捕获各自的 sessionId 与 bindingGeneration。不能将 sessionId 用作 xterm 宿主的 Vue key，也不能接到 failed/ended 就删除对应终端。

## 3. 命令清单

以下为目标命令名，M1/M2 按需实现、登记 ACL；不是一次全部暴露。

| 命令 | 主要参数 | 返回/语义 |
| --- | --- | --- |
| `available_session_kinds` | 无 | 注册工厂与能力，仅展示可用类型 |
| `open_session` | protocolVersion、requestId、target、size、onOutput、onControl | Created 快照；连接异步进行；同 owner/requestId 幂等 |
| `write_session` | sessionId、inputSeq、bytes | acceptedSeq；只确认后端有序接收，不承诺远端已执行 |
| `resize_session` | sessionId、size | 最新接受尺寸；合并重复 |
| `ack_session_output` | sessionId、consumedOffset（十进制字符串） | void；累计确认解析后的输出 |
| `respond_host_key` | sessionId、challengeId、decision | 接受/拒绝响应；decision 为 reject/allowOnce/trustAndSave |
| `respond_auth_challenge` | sessionId、challengeId、answers、savePolicy | masked 答案只短暂经 IPC；不得记录 |
| `cancel_auth_challenge` | sessionId、challengeId | 取消认证并关闭本次连接 |
| `close_session` | sessionId、reason | 受理后的当前快照；资源结束由终态事件确认 |
| `list_sessions` / `get_session` | 可选 sessionId | 当前 owner 的快照/终态缓存 |
| `session_client_heartbeat` | 无或已拥有 session ID 列表 | 窗口级失联检测；不能由前端指定 owner |
| `list_hosts` / `save_host` / `delete_host` | 查询/版本化 Host DTO/id | 参数化 repository CRUD；save 使用 revision 乐观锁 |
| `list_host_groups` / `save_host_group` / `delete_host_group` | 分组 DTO/id | 防环；默认移动子项后删除 |
| `list_known_hosts` / `revoke_known_host` | endpoint 查询/信任记录 id | 替换策略需二次显式操作，不在连接流程静默覆盖 |
| `select_private_key_file` | 原生文件选择 | opaque keyFileRef、名称、公钥元数据（可解析时）；不返回私钥 |
| `list_credentials` / `delete_credential` | id | 仅元数据；无通用 get_secret 命令 |
| `get_settings` / `save_settings` | 版本化设置 DTO | 校验范围；无原始 SQL |

keyFileRef 由后端文件选择流程签发并绑定 owner，持久化后转为受控文件引用；普通 invoke 不允许任意路径读取。密钥授权、状态变更与用户操作关联，不能仅相信“这个路径来自文件选择框”的前端声明。

## 4. 控制事件

事件统一含 protocolVersion、requestId、sessionId、eventSeq；`eventSeq` 在控制 Channel 内递增，`stateRevision` 防止迟到快照覆盖新状态。

| 事件 type | 负载 | 行为 |
| --- | --- | --- |
| `stateChanged` | snapshot | 更新 UI 元数据 |
| `hostKeyChallenge` | challengeId、endpoint、algorithm、fingerprint、expiresAt | 唯一活动 challenge；完整 key 留在后端 |
| `authChallenge` | challengeId、name、instructions、prompts[{text,echo}]、expiresAt | 普通文本显示；按 echo 控制输入 |
| `notice` | code、messageKey、脱敏 details | 限流、合并重复提示 |
| `ended` | finalState、reason、exitCode/exitSignal、finalOffset、outputComplete、error? | 无新输出生成；按 finalOffset 协调前端排空 |

首次输出可能晚于 Connected 控制事件；`ended` 也可能先于最后数据帧被 JS 收到。前端不得看到 ended 就直接销毁输出 handler，要消费至 finalOffset 或达到关闭排空期限。显式关闭允许 outputComplete=false 并明确舍弃剩余显示。

finalOffset 为 forwarder 停止后冻结的最后一次成功提交 Channel 的累计 payload 偏移；ended 发出后不再提交新数据帧。未发送的队列尾部不计入 finalOffset，并导致 outputComplete=false。outputComplete=true 只保证应用已接收输出均被提交，不等于 JS 已解析，也不保证远端网络之外尚未送达的数据；终态后的合法 ACK 由短期流摘要继续处理到排空/期限结束。

### 重连时复用同一终端

1. 保留 Tab、terminalId、xterm 和历史；禁止远端输入，清空旧待发送输入。重连按钮 single-flight；取消旧的未完成连接尝试。
2. 旧流已 ended 时等待其连续解析/排空，最长使用 3 秒初始预算；超时只放弃尚未显示的尾数据并显示不完整提示，不移除已显示内容。旧 actor 仍活动则先请求取消/关闭，不能与新流并发写同一个终端。
3. 终端渲染队列等待旧 write callbacks 的有界栅栏；处理旧连接留下的未闭合转义/UTF-8 与终端输入模式，再追加可信本地断开/重连状态分界。所选 xterm release 的安全边界处理方案需要 SP-07 验证，不能以调用 reset/clear 或重建实例来规避历史保留要求。
4. 更新 bindingGeneration，发起全新的 open_session；新帧 frameSeq/endOffset、输入 inputSeq 都从起点开始，输出仍写入原 xterm。前端尺寸取原终端当前有效尺寸。
5. 对旧 session 的迟到回调，禁止再追加内容或改新状态；可按旧 session 短期摘要 ACK 已解析尾部，不能把旧 offset 确认给新 session。新连接失败只更新状态并追加本地诊断，不清历史、不关 Tab。

若旧 xterm write 队列无法按期结束，显示可操作错误并保持历史，允许稍后重试；不能启动第二个流覆盖旧流。业务终态、后端 tombstone 过期与 Tab 生命周期相互独立。

## 5. 输出二进制帧 v1

一个 Channel 消息对应一帧，不依赖 TCP 包边界。一条输出 Channel 只绑定一个 session，不在每帧重复长 ID。头部固定 24 字节，小端序：

| 偏移 | 长度 | 字段 | 约束 |
| --- | --- | --- | --- |
| 0 | 1 | version | 1 |
| 1 | 1 | flags | v1 为 0；未知位拒绝 |
| 2 | 2 | reserved | 0 |
| 4 | 4 | frameSeq | u32，从 1 递增，不在同一 session 内回绕 |
| 8 | 8 | endOffset | u64；本帧之后的累计 payload 字节数 |
| 16 | 4 | payloadLength | u32，1–32768 |
| 20 | 4 | reserved | 0 |
| 24 | N | payload | 原始终端字节，不做换行或字符转换 |

startOffset = endOffset - payloadLength；必须等于上一帧 endOffset。JS 用 DataView 读 u64（BigInt），传回 JSON ACK 时转十进制字符串，不把 u64 塞入 JS number。frameSeq 耗尽前显式终止并提示，而非回绕后误判；这是长时间极端吞吐场景的边界，后续可升级协议。

先校验版本、长度、连续性和上限，再把 payload Uint8Array 交给 xterm。发现缺帧、重复、不合法偏移进入 PROTOCOL_MISMATCH 并关闭，不在终端流里静默“自愈”。不使用 String::from_utf8_lossy 或跨帧独立 TextDecoder 转换；xterm 自己处理 UTF-8 流。

## 6. 端到端流控

```text
SSH channel + 内部有限缓冲
  → driver（达到字节预算暂停取输出，仍处理取消）
  → 有界 output queue
  → forwarder（受未确认 payload 信用控制）
  → raw Channel
  → 前端有限待解析队列
  → xterm.write(bytes, callback)
  → callback 后累计 ACK
  → 释放发送信用
```

初始调参值，不是测试结果：

| 参数 | 初始值 | 说明 |
| --- | --- | --- |
| 单帧 payload | 4–32 KiB；不足不等待凑满 | 交互尾部约 4 ms 内刷新，实际待测 |
| 未 ACK 输出信用 | 256 KiB / session | 帧发送前预留信用；Tauri send 成功不释放信用 |
| driver 输出队列 | 512 KiB / session | 字节信号量 + 消息数双限，不能仅限制消息数量 |
| 输入队列 | 64 KiB / session | 超过限额暂停粘贴或报错，不静默丢字节 |
| 单次 write payload | 最大 16 KiB | 前端分片，单 writer 串行 invoke |
| ACK 批次 | 累计 64 KiB 或约 16 ms | 以 write callback 为依据；定时器受后台节流影响 |
| 已连接会话上限 | 默认 20；并行连接尝试 4 | 终态摘要不占活动配额；用户后续可调 |
| 全局应用流缓冲预算 | 32 MiB | 包含输出队列/信用/输入配额，独立于 xterm scrollback |

`sentOffset - consumedOffset <= outputCredit`。累计 ACK 必须单调、<= 已发送最大 offset、落在合法连续消费位置；重复 ACK 可忽略，未来 ACK/跨 session ACK 拒绝。后端用已发送帧端点确认合法边界。前端必须只确认**连续**已经完成解析的字节，不能对未解析帧提前 ACK。

frame header、序列化临时副本、russh 内部窗口/缓冲、WebView 和 xterm scrollback 是额外开销，不能用上表声称整个应用最多 32 MiB。库内部缓冲在 SP-02/05 中测量并配置上限。

没有 ACK 时 forwarder 停止送更多帧，不能无限填充 WebView；满队列后反压至 SSH channel，必要时影响该远端 Shell 的输出速度，这是保护内存的正常行为。协议 KeepAlive 和取消必须有独立运行路径。

## 7. 输入顺序、二进制与不确定交付

xterm.onData 的字符串用 TextEncoder 编码；onBinary 如启用，按单字节代码单元映射，不能再做 UTF-8 编码。M2 只声明 UTF-8 终端，不承诺任意传统编码自动识别。

前端每个 session 一个 FIFO，inputSeq 从 1 递增，一次只等待一个 write invoke；合并相邻小键入但不改变顺序。后端拒绝跳号，对已接受相同 seq 的重复请求返回原 ACK，对内容不一致的重复请求报协议错误。缓存按有界窗口保留；不得依靠无限输入历史去重。

inputSeq 同样使用 u32，不允许回绕；初始只缓存最近 128 条且总计最多 64 KiB 的已接受 payload 用于重复内容核对，超出任一上限淘汰旧条目。落在淘汰区间的旧 seq 直接拒绝，不重新写入。前端正常路径不重试写请求，序号耗尽时明确结束会话；后续需要更长寿命时升级协议。

write ACK 仅说明后端接纳；中途失联时输入可能已经部分送到远端。无 ACK 的请求不自动重试，不跨重连复用 inputSeq 或待发送队列。后端处理部分写和错误并汇报 INPUT_DELIVERY_UNKNOWN，绝不以失败为由重放命令。

## 8. 后台标签、失联与关闭

隐藏标签继续解析/ACK；只减少渲染与无意义 fit，不暂停整个消费管线。WebView 挂起可能延迟 ACK，先背压；初始 60 秒无消费进度显示 stalled，窗口确认存活时允许宽限至 5 分钟，之后以 CONSUMER_STALLED 取消。控制面窗口心跳初始 15 秒，90 秒失联进入疑似状态，仍适用 5 分钟宽限；睡眠唤醒需重置检测窗口，不能把系统睡眠直接判为协议错误。这些时间要在真实后台节流环境中调优。

Channel send 错误、窗口已销毁或主动 close 立即取消对应流，释放信用等待与网络任务。正常退出可排空至 finalOffset，显式关闭在 3 秒预算后可放弃显示尾部，标记 outputComplete=false。v1 不支持热重载后重新 attach 原 stream；UI 清楚提示需要重新连接。

## 9. 边界校验与版本演进

所有命令 Rust 端校验 owner、状态、长度、尺寸和速率；JS 类型不能代替运行时检查。协议版本不匹配先拒绝连接，未来优先兼容增加可选字段；改变二进制布局或状态语义必须升级版本。命令名、事件名、类型与错误码由中心文件管理；后续考虑代码生成，但不为了生成工具延迟首期。

## 10. 快捷键、配色与文件领域扩展

M2 的版本化设置增加 keymap 与终端默认 scheme 引用。规划 `list_terminal_schemes` / `save_terminal_scheme` / `delete_terminal_scheme` / `import_terminal_schemes` / `export_terminal_scheme`，后端校验字段/颜色/版本/引用与 revision；导入只接受原生选定文件或受大小限制的文本，不提供任意文件读写。快捷键只引用 CommandRegistry 已注册 commandId，链接动作走 `open_external_url` 的 http/https 白名单和用户手势校验，不能传任意 opener 参数。

M3 的端点、列表、属性、传输、冲突、原生拖拽使用独立命令族，完整草案见 [16 的 IPC 清单](16-sftp-workspace.md#8-受控-ipc-草案)。FileEndpoint/entryRef/transferId 与 sessionId 分属资源域，均检验 owner；SFTP-only 认证使用 connection 归属，不伪造终端 session。文件正文留在 Rust，列表分页、进度节流走独立类型化 Channel，不复用终端 binary frame/ACK，也不把文件 bytes 放入 Vue 状态。订阅事件 revision 与快照协调，窗口失联时队列状态保留有界，明确取消/租约策略。

原生拖入授权从原生事件建立，拖出只接受后端准备的 staging ref；普通 JS 路径不能冒充用户文件授权。所有新命令按实际里程碑纳入 AppManifest/ACL，SFTP 命令族当前没有注册或实现。

## 11. 首期实际接口补充（2026-10-15）

以 [Rust 命令](../src-tauri/src/commands/mod.rs) 与 [TS 门面](../src/services/desktop/client.ts) 为实际边界。`open_session` 额外接收 `terminalId`，快照包含 terminalId；owner 只取注入窗口身份。`write_session` 返回 `{acceptedSeq}`；`resize_session` 返回接受的 `{cols,rows}`。`save_host({host,expectedRevision})` / `save_host_group({group,expectedRevision})` 使用乐观锁，删除携带 revision。

`respond_auth_challenge` 当前只接受 `savePolicy:"never"`；保存秘密为主机编辑的显式 `save_credential({kind,label,secret}) -> {id}`，无读取秘密给 UI 的命令；交互 OTP 不保存。新增原生 `export_json({name,value})` 保存对话框、历史 `list_history` / `clear_history` 与中断凭据 `recover_credential({id})`。私钥选择可取消返回 null，只返回 keyFileRef/name。

设置通过 `get_settings` / `save_settings({settings})` 操作版本 1 的整体 app JSON；包含 customSchemes、hostSchemes 和 keymap，尚无独立配色命令族。拒绝未来设置版本和非法动作/颜色。AppManifest、handler、capability 共用中心命令名单并做静态集合检查；后续仍须实际验证 ACL 拒绝路径。

输出 forwarder 结束后发布 ended，终态 ACK 保留 3 秒排空窗口；会话快照/幂等摘要最多 1024 个、最长 10 分钟。五分钟未消费/窗口失联取消，系统睡眠时间跳跃重置失联预算。更细的 60 秒 stalled/90 秒 suspected UI 尚未实现。前端每 64 KiB 或 16 ms 合并解析后 ACK。各预算是代码初值，不是性能测量结果。

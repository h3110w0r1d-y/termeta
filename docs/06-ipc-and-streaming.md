# IPC 契约与终端数据流

日期：2026-10-15。协议版本 1，首期命令/Channel/ACL 已落源码，尚未运行验证。Rust/TS 实际入口及差异见 [需求与实现状态](01-product-requirements.md#8-当前实现与限制)。

## 1. 控制面与数据面

控制面采用 Tauri Commands + `Channel<SessionEvent>`，负责状态、challenge 和终态。数据面采用 `Channel<ArrayBuffer>` 接收 Rust 原始响应；输出 bytes 通过 `tauri::ipc::Response` 等锁定 release 支持的 raw Channel 方式发送。原始 ArrayBuffer 的支持依据见 [Tauri Channel API](https://v2.tauri.app/reference/javascript/api/namespacecore/)，具体实现需 SP-02 验证。

不使用全局终端 output event、不发 base64、不依赖 HTTP/WebSocket、不每个字符调用一次 IPC。两个 Channel 各自保持消息顺序，但不存在跨 Channel 的全局顺序，应用协议必须处理交错。

## 2. DTO 草案

```ts
type SessionKind = "ssh" | "local" | "serial";
type SessionState =
  | "created"
  | "connecting"
  | "awaitingHostKey"
  | "authenticating"
  | "awaitingAuth"
  | "opening"
  | "connected"
  | "closing"
  | "closed"
  | "failed"
  | "disconnected";

interface TerminalSize {
  cols: number;
  rows: number;
}
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
  | { source: "local" }
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
  | { type: "privateKey"; credentialId: string };
```

OpenTarget 接受保存/临时 SSH 与 `{source:"local"}`，Local 不接受任意命令、路径、环境或认证字段。available_session_kinds 返回 ssh/local 对应工厂能力，SessionSnapshot.kind 同步支持 local；本地复用现有 open/write/resize/ack/close 与ACL/owner/解锁校验。Serial 尚未开放。未传 password ref 时由认证 challenge 获取秘密，open_session 不需要明文密码字段。

前端另有不参与原生授权的稳定视图绑定：`TerminalBinding { terminalId, currentSessionId?, bindingGeneration }`。terminalId 在工作区创建时生成，重连不变；后端 sessionId 每次连接新建。前端 registry 按 terminalId 持有 xterm，session/channel 回调捕获各自的 sessionId 与 bindingGeneration。不能将 sessionId 用作 xterm 宿主的 Vue key，也不能接到 failed/ended 就删除对应终端。

## 3. 命令清单

以下为目标命令名，M1/M2 按需实现、登记 ACL；不是一次全部暴露。

| 命令                                                         | 主要参数                                                      | 返回/语义                                                   |
| ------------------------------------------------------------ | ------------------------------------------------------------- | ----------------------------------------------------------- |
| `available_session_kinds`                                    | 无                                                            | 注册工厂与能力，仅展示可用类型                              |
| `open_session`                                               | protocolVersion、requestId、target、size、onOutput、onControl | Created 快照；连接异步进行；同 owner/requestId 幂等         |
| `write_session`                                              | sessionId、inputSeq、bytes                                    | acceptedSeq；只确认后端有序接收，不承诺远端已执行           |
| `resize_session`                                             | sessionId、size                                               | 最新接受尺寸；合并重复                                      |
| `ack_session_output`                                         | sessionId、consumedOffset（十进制字符串）                     | void；累计确认解析后的输出                                  |
| `respond_host_key`                                           | sessionId、challengeId、decision                              | 接受/拒绝响应；decision 为 reject/allowOnce/trustAndSave    |
| `respond_auth_challenge`                                     | sessionId、challengeId、answers、savePolicy                   | masked 答案只短暂经 IPC；不得记录                           |
| `cancel_auth_challenge`                                      | sessionId、challengeId                                        | 取消认证并关闭本次连接                                      |
| `close_session`                                              | sessionId、reason                                             | 受理后的当前快照；资源结束由终态事件确认                    |
| `list_sessions` / `get_session`                              | 可选 sessionId                                                | 当前 owner 的快照/终态缓存                                  |
| `session_client_heartbeat`                                   | 无或已拥有 session ID 列表                                    | 窗口级失联检测；不能由前端指定 owner                        |
| `list_hosts` / `save_host` / `delete_host`                   | 查询/版本化 Host DTO/id                                       | 参数化 repository CRUD；save 使用 revision 乐观锁           |
| `list_host_groups` / `save_host_group` / `delete_host_group` | 分组 DTO/id/revision，删除可选 deleteHosts                     | 防环；默认保留上移，true 原子删除子树及主机                                    |
| `list_known_hosts` / `revoke_known_host`                     | endpoint 查询/信任记录 id                                     | 替换策略需二次显式操作，不在连接流程静默覆盖                |
| `select_private_key_file`                                    | 原生文件选择                                                  | opaque keyFileRef、名称、公钥元数据（可解析时）；不返回私钥 |
| `list_credentials` / `delete_credential`                     | id                                                            | 仅元数据；无通用 get_secret 命令                            |
| `get_settings` / `save_settings`                             | 版本化设置 DTO                                                | 校验范围；无原始 SQL                                        |

keyFileRef 由后端文件选择流程签发并绑定 owner，作为导入私钥凭据的受控文件引用；普通 invoke 不允许任意路径读取。密钥授权、状态变更与用户操作关联，不能仅相信“这个路径来自文件选择框”的前端声明。

## 4. 控制事件

事件统一含 protocolVersion、requestId、sessionId、eventSeq；`eventSeq` 在控制 Channel 内递增，`stateRevision` 防止迟到快照覆盖新状态。

| 事件 type          | 负载                                                                         | 行为                                      |
| ------------------ | ---------------------------------------------------------------------------- | ----------------------------------------- |
| `stateChanged`     | snapshot                                                                     | 更新 UI 元数据                            |
| `hostKeyChallenge` | challengeId、endpoint、algorithm、fingerprint、expiresAt                     | 唯一活动 challenge；完整 key 留在后端     |
| `authChallenge`    | challengeId、name、instructions、prompts[{text,echo}]、expiresAt             | 普通文本显示；按 echo 控制输入            |
| `notice`           | code、messageKey、脱敏 details                                               | 限流、合并重复提示                        |
| `ended`            | finalState、reason、exitCode/exitSignal、finalOffset、outputComplete、error? | 无新输出生成；按 finalOffset 协调前端排空 |

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

| 偏移 | 长度 | 字段          | 约束                                    |
| ---- | ---- | ------------- | --------------------------------------- |
| 0    | 1    | version       | 1                                       |
| 1    | 1    | flags         | v1 为 0；未知位拒绝                     |
| 2    | 2    | reserved      | 0                                       |
| 4    | 4    | frameSeq      | u32，从 1 递增，不在同一 session 内回绕 |
| 8    | 8    | endOffset     | u64；本帧之后的累计 payload 字节数      |
| 16   | 4    | payloadLength | u32，1–32768                            |
| 20   | 4    | reserved      | 0                                       |
| 24   | N    | payload       | 原始终端字节，不做换行或字符转换        |

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

| 参数               | 初始值                   | 说明                                                |
| ------------------ | ------------------------ | --------------------------------------------------- |
| 单帧 payload       | 4–32 KiB；不足不等待凑满 | 交互尾部约 4 ms 内刷新，实际待测                    |
| 未 ACK 输出信用    | 256 KiB / session        | 帧发送前预留信用；Tauri send 成功不释放信用         |
| driver 输出队列    | 512 KiB / session        | 字节信号量 + 消息数双限，不能仅限制消息数量         |
| 输入队列           | 64 KiB / session         | 超过限额暂停粘贴或报错，不静默丢字节                |
| 单次 write payload | 最大 16 KiB              | 前端分片，单 writer 串行 invoke                     |
| ACK 批次           | 累计 64 KiB 或约 16 ms   | 以 write callback 为依据；定时器受后台节流影响      |
| 已连接会话上限     | 默认 20；并行连接尝试 4  | 终态摘要不占活动配额；用户后续可调                  |
| 全局应用流缓冲预算 | 32 MiB                   | 包含输出队列/信用/输入配额，独立于 xterm scrollback |

`sentOffset - consumedOffset <= outputCredit`。累计 ACK 必须单调、<= 已发送最大 offset、落在合法连续消费位置；重复 ACK 可忽略，未来 ACK/跨 session ACK 拒绝。后端用已发送帧端点确认合法边界。前端必须只确认**连续**已经完成解析的字节，不能对未解析帧提前 ACK。

frame header、序列化临时副本、russh 内部窗口/缓冲、WebView 和 xterm scrollback 是额外开销，不能用上表声称整个应用最多 32 MiB。库内部缓冲在 SP-02/05 中测量并配置上限。

没有 ACK 时 forwarder 停止送更多帧，不能无限填充 WebView；满队列后反压至 SSH channel，必要时影响该远端 Shell 的输出速度，这是保护内存的正常行为。协议 KeepAlive 和取消必须有独立运行路径。

## 7. 输入顺序、二进制与不确定交付

xterm.onData 的字符串用 TextEncoder 编码；onBinary 如启用，按单字节代码单元映射，不能再做 UTF-8 编码。M2 只声明 UTF-8 终端，不承诺任意传统编码自动识别。

前端每个 session 一个 FIFO，inputSeq 从 1 递增，一次只等待一个 write invoke；合并相邻小键入但不改变顺序。后端拒绝跳号，对已接受相同 seq 的重复请求返回原 ACK，对内容不一致的重复请求报协议错误。缓存按有界窗口保留；不得依靠无限输入历史去重。

inputSeq 同样使用 u32，不允许回绕；初始只缓存最近 128 条且总计最多 64 KiB 的已接受 payload 用于重复内容核对，超出任一上限淘汰旧条目。落在淘汰区间的旧 seq 直接拒绝，不重新写入。前端正常路径不重试写请求，序号耗尽时明确结束会话；后续需要更长寿命时升级协议。

write ACK 仅说明后端接纳；中途失联时输入可能已经部分送到远端。无 ACK 的请求不自动重试，不跨重连复用 inputSeq 或待发送队列。后端处理部分写和错误并汇报 INPUT_DELIVERY_UNKNOWN，绝不以失败为由重放命令。

## 8. 后台标签、失联与关闭

隐藏标签继续解析/ACK，只减少渲染与无意义 fit。WebView 挂起时先背压，五分钟未消费或窗口失联取消；控制面心跳用于发现失联，系统睡眠时间跳跃重置预算。具体阈值需在真实后台节流环境调优。

Channel send 错误、窗口已销毁或主动 close 立即取消对应流，释放信用等待与网络任务。正常退出可排空至 finalOffset，显式关闭在 3 秒预算后可放弃显示尾部，标记 outputComplete=false。v1 不支持热重载后重新 attach 原 stream；UI 清楚提示需要重新连接。

## 9. 边界校验与版本演进

所有命令 Rust 端校验 owner、状态、长度、尺寸和速率；JS 类型不能代替运行时检查。协议版本不匹配先拒绝连接，未来优先兼容增加可选字段；改变二进制布局或状态语义必须升级版本。命令名、事件名、类型与错误码由中心文件管理；后续考虑代码生成，但不为了生成工具延迟首期。

## 10. 快捷键、配色与文件领域扩展

快捷键及终端配色存入版本化设置，通过 get_settings 读取完整快照、save_settings 提交局部字段。后端校验动作、颜色、版本及引用；前端 JSON 导入有大小限制，不提供任意文件读写。快捷键只引用 CommandRegistry 已注册 commandId，链接动作走 open_external_url 的 http/https 白名单，不接受任意 opener 参数。

文件服务使用独立命令与进度 Channel，不复用终端 binary frame/ACK。正文只在 Rust 流动。当前单终端 SFTP 契约见第 12 节；独立端点、连接租约、持久任务订阅仍是后续设计。

## 11. 数据、设置与控制接口

以 [Rust 命令](../src-tauri/src/commands/mod.rs) 与 [TS 门面](../src/services/desktop/client.ts) 为实际边界。`open_session` 额外接收 `terminalId`，快照包含 terminalId；owner 只取注入窗口身份。`write_session` 返回 `{acceptedSeq}`；`resize_session` 返回接受的 `{cols,rows}`。`save_host({host,expectedRevision,password?})` / `save_host_group({group,expectedRevision})` 使用乐观锁，删除携带 revision。

`respond_auth_challenge` 当前只接受 `savePolicy:"never"`；公共凭据保存为凭据页的显式 `save_credential({kind,label,password?,keyFileRef?,passphrase?}) -> {id}`，无读取秘密给 UI 的命令；交互 OTP 不保存。新增原生 `export_json({name,value})` 保存对话框、历史 `list_history` / `clear_history`。私钥选择可取消返回 null，只返回 keyFileRef/name；该引用用于导入 privateKey 凭据，不用于会话配置。

`save_host.password` 省略/null 表示元数据更新保留已有专属密码，显式 `""` 表示移除，非空表示替换；主机编辑总显式提交字符串。HostDraft 不含秘密，主机公共结果新增 hasPassword 且隐藏专属credentialId。私钥凭据参数只接受 keyFileRef 与可选passphrase，密码凭据只接受password，不接受混合类型；所有secret只作为提交参数，不回传。保存主机后的旧秘密清理失败不回滚已经提交的主机结果，deleting登记留待重试。

设置通过 `get_settings` / `save_settings({settings, defaults})` 操作版本 1 的 app JSON（settings为局部更新，defaults为首次无记录时使用的完整默认值，二者均受校验）；包含 customSchemes、hostSchemes、keymap 和 tabPosition（top/left，旧记录缺失兼容），尚无独立配色命令族。拒绝未来设置版本和非法动作/颜色。AppManifest、handler、capability 共用中心命令名单并做静态集合检查；后续仍须实际验证 ACL 拒绝路径。

输出 forwarder 结束后发布 ended，终态 ACK 保留 3 秒排空窗口；会话快照/幂等摘要最多 1024 个、最长 10 分钟。五分钟未消费/窗口失联取消，系统睡眠时间跳跃重置失联预算。前端每 64 KiB 或 16 ms 合并解析后 ACK。各预算是代码初值，不是性能测量结果。

认证提交成功后前端只移除匹配 sessionId/challengeId 的提示，不能按 sessionId 删除随后到达的下一轮 OTP；会话终态/关闭仍清除该会话全部提示。自动 MFA 复用现有 AuthChallenge，无新增 IPC 命令。后端自动处理空白零输入轮与唯一明确密码提示的一次保存密码响应，不向前端发送该轮 AuthChallenge 或密码正文；非空通知、OTP 和其他提示仍走现有事件。自动/人工轮次共用提示校验和挑战轮数上限。

终端字体：`app.fontFamily` 保存固定12项规范名称，名单见 [桌面字体设计](09-desktop-ux.md#6-字体主题与可访问性)。`save_settings` 拒绝名单外值；`get_settings` 对旧版有效字符串但不在名单中的值，仅在返回值中归一为 DejaVu Sans Mono，其余字段仍完整校验，不执行自动持久化。字体二进制从应用自带静态资源加载，不通过 IPC 返回字体正文或访问系统字体目录。

内置配色：64项内置 ID 从 resources/terminal-palettes.json 读取，前后端共享；保存设置仍拒绝未知或悬空引用。get_settings 对移除的旧预设引用只归一返回值：dark/forest/amber/violet/contrast-dark → nord-dark，light/contrast-light → nord-light；ocean 采用同 ID 的提取配色。未修改用户自定义方案 ID，不自动写库。

工作区广播：复用每目标原有write_session和FIFO，不增加广播IPC命令或跨会话事务。输入只复制到当前组已连接目标，每目标编号/限额/错误独立；源只写一次。粘贴确认/剪贴板等待期校验组广播版本与会话代次，旧输入不跨重连；任一目标失败不重放，也不保证多机同时执行。

## 12. 单终端 SFTP

5 个命令均注册于 command_names/AppManifest、invoke_handler 和 main capability，逐次校验本地 main 窗口、owner、session 归属及状态。取消可用于结束中的会话，以便释放旧任务。

| 命令             | 入参                                                             | 返回                                              |
| ---------------- | ---------------------------------------------------------------- | ------------------------------------------------- |
| sftp_list        | sessionId、path、more                                            | DirectoryPage { path, entries, hasMore }          |
| sftp_transfer    | sessionId、transferId、directory、name、upload、onProgress       | bool；上传原生多选，下载单文件保存对话框          |
| sftp_drop_upload | sessionId、transferId、dropId、directory、onProgress             | bool；消费原生拖入授权                            |
| sftp_start_drag  | sessionId、transferId、directory、names、directories、onProgress | bool；macOS 原生文件/目录承诺，其他平台返回不支持 |
| sftp_cancel      | sessionId、transferId                                            | void；只取消匹配任务，旧 ID 不取消后续任务        |

transferId 为 UUID。FileEntry 包含 name、kind(file/directory/symlink/other)、linkTarget、targetKind、size(十进制字符串或 null)、modified(u32 秒或 null)、permissions(u32 或 null)。链接目标和类型查询失败可独立为空，不改写链接自身元数据。命令范围仅浏览与上传下载。

目录每页最多 100 个远端条目，跨 READDIR 包补齐并缓存余项；hasMore 表示缓存或句柄尚有后续，必要时空页确认 EOF。前端的置顶 `..` 不计分页，根目录禁用。链接元数据只查本页，每批最多 8 项；逻辑路径保留 `/bin` 等链接前缀，折叠 `.`、`..` 与重复斜杠，相对路径以服务器 REALPATH(".") 为基准，OPENDIR 和后续文件访问直接使用逻辑绝对路径。

TransferProgress 包含 transferred/total 字节十进制字符串及可选 currentName/completedFiles/totalFiles。字节为当前文件进度，计数为当前批次；不传正文或本地路径。普通传输约 150 ms 节流，命令成功才标完成。上传预扫描最多 64 层/10000 项，前端最多等待 32 批次。文件覆盖由 Rust 原生逐项确认，目录合并；失败不回滚已完成文件。

原生拖入签发一次性 dropId，绑定 owner，5 分钟有效，最多 16 个引用、每批 1000 根路径；满额淘汰最旧未消费项。事件 sftp-local-hover {position}、sftp-local-drop {dropId,position,error?}、sftp-local-leave 不泄露本地路径。position 为壳归一后的 WebView 逻辑坐标，前端不得再次除 DPR；超限返回错误。仅当前可见面板消费，目标固定当前目录，确认目标后提交。上传授权须有带真实路径的原生Enter与后续Drop配对，空路径或未配对的Drop直接忽略；拖出期间软件内Drop直接取消原生拖出命令，不能成为上传。排队过期须重新拖入，没有任意本地路径读取 IPC。

macOS 拖出 names 最多 100 个普通文件或真实目录根，directories 必须为其子集且仅为类型提示，Rust 在兑现时复核。原生手势启动须左键仍按住，不能排队到手势结束后。Finder 请求内容才发第一条进度并创建下载记录；无内容请求的取消不留任务。内部落点拒绝，原生拖出期间屏蔽上传事件及授权登记。命令在 Copy 会话结束且全部承诺完成后成功返回；取消返回 false，失败返回错误。目录使用独立句柄有界递归，不扰动侧栏分页，不跟随源链接。

传输始终绑定原 sessionId，隐藏不取消；断线/关闭终端取消，重连不重放或关联新会话。尚无持久任务恢复、通用冲突 challenge、独立端点或连接租约。以上仅为源码契约，未完成编译与平台运行验收。

双栏文件工作区增加 `sftp_connect(profileId, requestId, onControl)`，返回 `SessionSnapshot`；连接属于文件专用会话表，不创建终端，复用现有控制事件及主机信任/认证响应。`sftp_copy(request, onProgress)` 的 request 为 sourceSessionId、targetSessionId、sourcePath、targetPath、names、transferId，返回空成功；只接受两个 owner-bound 文件会话，正文不进入IPC。列表和取消复用 `sftp_list` / `sftp_cancel`，关闭复用 `close_session`；认证与关闭根据资源所属会话表路由。所有新增命令同步 AppManifest/ACL/main capability。

## 数据保险库解锁命令（2026-10-16）

`vault_status() -> {mode: "setup"|"locked"|"unlocked", keychainWarning: string|null}` 每进程最多自动尝试一次系统主密钥；`unlock_vault({password: string, setup: boolean})` 在Rust阻塞池派生并校验，成功后返回同一状态DTO。二者均校验main窗口及本地导航，加入AppManifest命令名单与main capability。密码/主密钥不出现在返回值或事件中。其他用例命令先检查保险库已解锁，否则返回VAULT_LOCKED；前端统一经desktop门面。

已移除旧`recover_credential`：secret与ready现在在同一加密数据库写入，未完成公共pending项由用户删除后重建。原有主机/凭据DTO保持不返回秘密。

分组删除契约（2026-10-16）：`delete_host_group(id, revision, deleteHosts?: boolean)` 沿用原命令与 main owner/保险库门禁/ACL，省略或 false 保留直属主机和子组并上移；true 按根组 revision 校验后，在同一数据库事务内从叶到根删除整个子树和主机、将专属凭据登记 deleting，再复用协调器清理。共享凭据不删除，已连接会话保持快照，不新增命令或 schema。前端选择 true 必须另弹最终确认。主机拖拽复用 `save_host` 并省略 password，分组拖拽复用 `save_host_group` 的 parentId/sortOrder/revision；无需新建移动接口。

主机页设置契约：现有AppSettings增加可选旧格式兼容的`hostsView`对象，新保存包含`includeChildren: boolean`与`expandedGroupIds: string[]`；Rust白名单校验字段、UUID去重及10000项/整条256KiB上限。复用get_settings/save_settings，不新增IPC/ACL。前端保护包含子组、展开集合及sftpShowHidden不被设置草稿回退。全局拼音搜索只用已有轻量HostProfile列表，无逐次搜索IPC或网络请求。

搜索偏好通过既有get_settings/save_settings保存app.hostSearch（pinyin布尔值与fields字段集合），name强制存在，其他字段为address/username/tags/group且去重；缺失兼容旧数据。搜索下拉与设置页即时保存共用设置队列，设置页只提交修改字段，无新增IPC或ACL。

新建标签页弹窗只使用既有HostProfile/设置，无新IPC。keymap.commandId增加workspace.newTab（application/workspace），Rust设置白名单同步；前端读取或导入旧keymap补充Primary+T绑定，保留已有动作/冲突配置及绑定数量上限。

## 独立设置窗口契约

`open_settings_window()`仅main可调用且要求已解锁，Rust创建/聚焦settings窗口。`get_settings`返回完整AppSettings；`save_settings`在设置专用异步互斥队列中读取最新快照、合并patch并校验后写库，返回完整快照。hostsView/hostSearch额外按子字段合并，其余字段整体替换；默认值仅在无记录时使用，旧版本拒绝写入，兼容旧字体/配色归一。成功后Rust广播无秘密的`settings-changed`失效通知，两个窗口串行重读；前端不发送设置事件。数据库schema及设置schemaVersion不变。

settings窗口仅开放设置和export_json，这些命令使用独立settings_owner检查窗口身份、本地来源和解锁状态；list_known_hosts/revoke_known_host/list_history/clear_history随业务页面移入保险箱，改用仅main可用的owner，且从settings capability移除。其余资源owner保持main限制。新增命令纳入COMMANDS/AppManifest、handler和main capability；settings使用独立capability。

## SSH 系统图标契约

HostDraft.icon为内置字符串标识，缺省ssh；HostProfile新增icon与systemDetectionAttempted:boolean。后端SshConfig.system_profile_id使用serde(skip)，只由open_session的saved分支设置，不接收前端资源归属；Local、ephemeral及独立SFTP均不设置。新增控制事件{type:"hostSystemChanged", profileId:string}，使用既有SessionEvent信封的sessionId/requestId/eventSeq，仅通知重读已提交主机列表；不传输SVG、任意脚本或探测输出。终端注册表保留代次检查，主机列表读取忽略迟到旧请求。

AppSettings新增detectSshSystem:boolean，旧设置缺失默认true；Rust验证仅接受布尔值，拒绝额外脚本字段。复用open_session/list_hosts/save_host/get_settings/save_settings及原ACL/owner，无新增命令或权限。

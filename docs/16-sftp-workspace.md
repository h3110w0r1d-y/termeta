# SFTP 侧栏、双栏工作区与文件传输

日期：2026-10-15。M3/P0 产品需求与技术契约，尚未实现。两种界面共享文件服务与传输队列；不把文件 API 加进通用 TerminalSession。

## 1. 两种入口

| 入口 | 布局与端点 | 生命周期 |
| --- | --- | --- |
| SSH 终端附属 SFTP 面板 | 可显示/隐藏，位置可选左侧、右侧或底部；默认右侧 | 绑定该 SSH 目标，独立 SFTP channel；隐藏不终止任务、不销毁 xterm |
| 独立 SFTP Tab | 左右双栏，各自可选本地或 SSH 主机，独立路径/选择/历史 | 不依赖终端 Tab；仅传文件也可建立 SSH，不申请 PTY 或启动 Shell |

附属面板支持拖动调整尺寸、目录导航、文件操作与队列入口；位置/默认显隐/尺寸可全局设置，并允许 profile 或当前视图覆盖。切位置只改变布局，稳定 browserId 与 terminalId 不变。折叠/显示触发终端实际有效尺寸的 fit/resize，不发送 0×0、不重建连接、不清空滚屏。面板断连时保留最后列表并标记“过期”，禁用有副作用的操作，重新绑定后刷新。

独立 SFTP Tab 两侧分别选择“本地”或保存的 SSH profile/快速连接，展示各自真实端点身份、连接状态、面包屑、目录列表与属性入口。切换一侧不改变另一侧；正在传输的任务使用创建时端点快照，不能被后续选机操作重定向。提供切换端点、交换左右视图、返回/前进/上级与刷新；交换视图不移动数据。

支持 Local↔Remote、Remote A↔Remote B、Local↔Local，以及同一远端不同目录复制。本地文件浏览器是文件服务能力，不依赖 M4 的 Local Terminal/PTY。

## 2. 文件浏览与属性

- 列表展示名称、类型、大小、修改时间与权限；支持隐藏文件、排序、多选、键盘导航、路径输入与刷新。大目录分页/增量读取并虚拟化，不能把完整目录无限塞进 DOM。
- 支持新建目录、重命名、复制、删除与查看属性；删除无远端回收站承诺，展示目标/数量并确认。递归操作先统计/预览，有界扫描，可取消；不默认跟随符号链接遍历。
- 属性可修改权限（八进制与 rwx 编辑器）、访问/修改时间；服务端与权限允许时修改 numeric uid/gid。用户名解析、ACL/xattr 与高级属性按能力另列，不假称各服务器均支持。
- 依据后端能力和实际权限显示可编辑项；服务端拒绝时展示真实错误。批量修改逐项报告成功/失败，不把部分成功显示为全部完成；递归修改另行确认范围，不默认 sudo。
- SFTP 属性操作通过协议完成，不偷偷运行 chmod/chown 或拼接 Shell 命令。符号链接属性/目标属性明确区分；不支持 link 自身修改时禁用并解释。
- 文件名保留后端原始身份，显示文本不成为操作路径。无法无损显示的字节名需转义并使用 opaque entryRef 操作；不承诺任意服务器文件名都是 UTF-8。

## 3. 拖入、拖出与内部拖拽

| 操作 | 产品行为 | 技术边界 |
| --- | --- | --- |
| 操作系统文件拖入远端列表 | 显示落点与端点，确认后上传；目录递归遵循策略 | Tauri 原生 drop 取得路径并在后端签发授权引用，不能相信普通 JS 字符串路径 |
| 远端文件拖到本地栏 | 下载到该栏当前目录 | 与“下载”按钮共用任务/覆盖流程 |
| 本地栏拖到远端栏 | 上传 | 与外部拖入共用任务/覆盖流程 |
| 远端 A 拖到远端 B | 一次操作发起远端到远端复制 | 默认客户端受界流式中转，用户无需手动下载再上传 |
| 远端/本地条目拖到 OS 文件管理器 | 原生拖出并复制文件到外部目标 | 属于明确交付需求，必须经过三平台原生拖出验证，不能以 Web HTML DnD 已可用代替 |

内部拖拽使用 owner-bound entryRef 和 endpointId；只传元数据，不把文件内容放入 dataTransfer/Pinia。默认是复制，拖拽成功不删除源文件。显式“移动”可后续作为独立动作，不能把跨主机复制+删除冒充原子 move。拒绝同文件复制到自身、目录复制到子目录；检查别名/符号链接后仍不确定时阻止并提示。

外部拖入与内部拖拽需要统一落点识别和高亮，校正缩放坐标与 WebView/native drop 竞争，避免触发两次上传。对目录、特殊文件、符号链接明确提示处理策略；设备/socket/FIFO 默认不上传。

**原生拖出是独立技术验证门槛 SP-11。** Tauri WebView 的 drag/drop 接口不等于支持“远端虚拟文件拖出”。候选路线是先下载到应用受控临时目录，准备完毕后由原生适配器开始拖出真实本地文件；准备期间显示进度，可取消，不对未完成的文件提前报告拖拽成功。文件 promise/虚拟文件模式仅在平台适配验证后启用，不向 OS 传 SSH URL 冒充本地路径，也不把大文件全量转为 JS Blob。

候选 `tauri-plugin-drag`/drag-rs 尚未安装或冻结版本；应验证 Windows Explorer、macOS Finder、Linux X11/Wayland 文件管理器、文件夹、多选、目标冲突、取消与沙箱路径。外部目标的覆盖由 OS 文件管理器流程决定，尊重其询问/取消；应用不能绕过目标系统的冲突处理。若目标环境不支持原生拖出，提供明确的“下载到…”与限制说明，该环境的拖出需求保持未通过，不能用替代按钮宣称已完成。

临时拖出文件可能包含秘密：仅用户发起后创建，限制磁盘配额、权限与有效期，取消/结束后在 OS 不再读取时清理；启动时只清理应用登记且确定过期的残留。不扫描删除用户目录；不承诺 SSD 安全擦除。需验证拖出结束通知与 OS 延迟读取，不能刚开始拖拽就删除文件。

## 4. 远端到远端与连接租约

```text
远端 A / SFTP reader
  → Rust 有界块缓冲 / TransferManager
  → 远端 B / SFTP writer
```

默认流经客户端内存，不在本地磁盘暂存整个文件，不依赖远端 scp/rsync/Shell，也不假设两个服务器能彼此联网。任一侧先完成主机身份核验与认证；后端复用已有合适连接租约或建立独立连接，UI 明示两个端点。未来服务器 copy-data 扩展只作为能力优化，不能作为跨主机传输默认前提。

SshConnectionService 管理 transport/channel/租约，SFTP Browser 与传输任务分别持有租约。隐藏面板不释放运行任务；关闭终端 Tab 时提示后台继续或取消关联传输。独立 SFTP Tab 关闭也允许在应用内后台队列继续，或明确取消；应用退出需统一确认。用户“断开终端”只关闭 Shell 租约，“断开该目标全部连接”明确取消相关文件任务和租约。

共享 SSH transport 故障时各子系统更新各自状态：终端保留历史，文件列表标过期，任务标失败/待用户处理。不因终端手动重连把旧文件任务自动挂到新身份，不自动重新执行覆盖操作。重新绑定需验证相同 endpoint/信任，刷新列表；失败任务允许明确重试，不默认断点续传。续传需额外核验源/目标版本、偏移和校验策略，单独实现后开放。

## 5. 传输任务与资源边界

任务状态为 `queued → preflight → awaitingConflict? → running → finalizing → completed`，任意阶段可按规则进入 `failed/cancelled`；等待冲突不阻塞无关任务。目录是父任务+子文件结果，整体部分失败时展示计数及失败项，不伪装全部成功。

初始预算（待 SP-10 调优）：全局并行文件 4 个，每 endpoint 2 个；块大小 64–256 KiB，每任务在途数据预算 2 MiB，全局传输缓冲预算 16 MiB。库内部请求窗口/缓冲另行计入，递归扫描与待处理任务数量也设上限，超过后按批扫描。不因队列满无限预读。传输与终端输出分开调度，限制文件占用以保证终端输入/KeepAlive/取消响应。

Rust 负责数据读写与累计进度，前端只接收节流状态（初始约每秒 5–10 次及关键状态立即发送）。字节数采用 u64，IPC 超过 JS 安全范围用十进制字符串。显示已传字节、总大小（未知时不伪造百分比）、速率、错误、取消与重试。取消停止新 I/O，释放 reader/writer/租约，清理本任务临时文件并报告清理失败；不能直接删最终目标或源文件。

## 6. 覆盖与冲突：传输前和发布前都检查

所有上传必须判断是否允许覆盖；下载、远端中转、复制/重命名同样执行一致规则。默认策略为“询问”，展示源/目标端点、完整路径、类型、大小/时间及风险；提供跳过、覆盖、重命名、取消。批量“本任务后续同类冲突采用此项”只作用于明确任务，不永久变成全局默认。目录合并不代表允许覆盖其中所有同名文件；文件/目录类型冲突单独处理。

1. `preflight` 检查源仍存在、目标父目录/权限、同名项类型和属性；用户无权写入时明确失败，不能预检通过就保证以后一定可写。
2. 无冲突也需防检查后被创建的竞态。尽可能使用 exclusive create/no-replace；SFTP 普通 rename、POSIX rename 扩展、服务器行为的覆盖语义不同，按锁定 API 与服务器能力验证。
3. 上传/中转优先写目标目录内唯一、独占创建且属于当前任务的 `.part` 文件，写完关闭并确认所需属性；发布前重新核验目标版本/冲突决策。目标变化重新询问，不能复用过时同意覆盖另一个文件。
4. 支持安全 atomic replace 时才使用并声明原子替换；不支持时不能承诺无竞态覆盖。以明确的兼容模式提示保证较弱并要求用户决定，或安全跳过/另存名称；默认不得“删旧文件再上传”。仅预先 stat 并不能提供跨服务器通用的原子条件替换。
5. 失败、配额满、断网或取消时，尽可能保留旧最终文件，只清理本任务确切创建的临时项。清理失败显示路径和原因；不得按 `.part` 通配符批量删除他人文件。

本地目标同样用安全创建/替换与路径授权。严格“仅当目标版本未改变才替换”在缺少服务端条件操作/锁时不能完全保证；实现必须说明能力与残余竞态，不能把弱保证标成严格模式。不得自动退回破坏性路径。完整性默认确认实际读写长度；协议不提供端到端文件哈希时不谎称已完成独立哈希校验。

## 7. 模型与后端模块

```ts
type FileEndpoint =
  | { type: "local"; endpointId: string; rootGrantId: string }
  | { type: "ssh"; endpointId: string; profileId?: string; connectionRef: string };

interface FileEntryRef {
  endpointId: string;
  entryRef: string; // 后端拥有：真实名称、路径与列表 revision
}
interface TransferPlanV1 {
  schemaVersion: 1;
  requestId: string;
  sources: FileEntryRef[];
  destination: { endpointId: string; directoryRef: string };
  conflictPolicy: "ask" | "skip" | "rename"; // 覆盖需单独 challenge 决策
}
```

`browserId` 标识文件视图，`endpointId` 标识已授权端点实例，`transferId` 标识任务，`challengeId` 标识一次冲突决策；均在 Rust 按窗口 owner 校验，随机 ID 本身不授予权限。任务保存端点/源/目标快照及 revision，不保存可变的“当前左栏”。本地可从已授权根向内导航；向其他根导航需原生选择或原生 drop 形成的新授权，不允许前端任意路径读写。

计划模块：

- `terminal-files`：协议无关的 FileEndpoint/Entry/Metadata/Capabilities、FileProvider 端口、TransferManager、冲突与取消模型；不依赖 Tauri/russh/SQLx。
- `terminal-sftp`：实现 FileProvider，经 terminal-ssh 的受控连接服务取得子系统流，使用 russh-sftp；不持有 Shell。
- 本地文件适配器：实现同一 FileProvider，处理本地根授权、文件句柄、安全创建/替换和符号链接；在 files 的平台适配模块隔离实现，不能依赖 portable-pty。
- `src-tauri`：签发授权、owner/ACL、原生 drop/drag/dialog、组装 provider 和进度 Channel；前端 `modules/files` / `modules/transfers` 复用 FileBrowser、属性与任务组件。

FileProvider 语义包含 list/stat/lstat、受界 openRead/openWrite、mkdir、rename、remove、setMetadata 与 capabilities；具体 Rust 对象安全/异步流签名在 M3 冻结。传输策略在 files，协议细节在适配器。files 不反向依赖 ssh；sftp 依赖 files + ssh；壳注入实现，避免循环依赖。

## 8. 受控 IPC 草案

| 命令族 | 输入/输出边界 | 校验 |
| --- | --- | --- |
| `open_file_endpoint` / `close_file_endpoint` | local 原生 root 授权或 SSH profile；返回端点/能力/状态 | owner、主机信任/认证、租约策略，无原始秘密回传 |
| `list_file_entries` / `stat_file_entry` | endpointId、directoryRef/cursor 或 entryRef | 路径授权、分页/长度限额、列表 revision |
| `create_file_directory` / `rename_file_entry` / `delete_file_entries` | 受控引用、合法新名称、确认 token | owner、符号链接语义、版本、递归范围、目标冲突 |
| `set_file_metadata` | entryRef、version、属性字段白名单 | 服务端能力/权限；不接受 shell 命令 |
| `plan_transfer` / `start_transfer` | 已授权 entryRefs/目标目录；返回计划/任务 | 两端授权、限额、快照、requestId 幂等；start 不跳过覆盖判断 |
| `respond_transfer_conflict` / `cancel_transfer` | transferId、challengeId、决策 | 所属窗口、有效期、源/目标版本；旧 challenge 拒绝 |
| `list_transfers` / `subscribe_transfers` | owner 的快照/节流进度 Channel | 有界待发状态、事件 revision；不可跨窗口订阅 |
| `prepare_file_drag` / `start_file_drag` | entryRefs、当前用户拖拽意图；受控 staging refs | 所有权、配额、平台能力；不接受任意本地路径 |

命令实现时全部纳入 Tauri 自定义 ACL；上表不是开放通用文件系统插件的授权。文件正文仅在 Rust provider 之间流动，不进入终端输出 Channel、WebView、DB 或诊断日志。端点认证复用 SSH challenge 机制，但使用明确资源类型/connectionId 归属，不能伪造 terminal sessionId。

## 9. 官方资料与验收门槛

- [russh-sftp SftpSession](https://docs.rs/russh-sftp/latest/russh_sftp/client/struct.SftpSession.html)：文件读写、open flags、属性等能力；存在 API 不等于远端支持所有扩展或权限操作。
- [Tauri WebView API](https://v2.tauri.app/reference/javascript/api/namespacewebview/)：原生 drop 事件能力参考；不能推断虚拟远端文件拖出已解决。
- [drag-rs 维护者仓库](https://github.com/crabnebula-dev/drag-rs) / [插件说明](https://docs.crabnebula.dev/plugins/drag-rs/)：仅原生拖出候选，版本/许可/维护状态/三平台行为待验证。

SP-10 必须覆盖独立 SFTP 无 Shell、左右两侧自由选端点、远端中转受界内存、面板三位置与隐藏保活、属性/权限错误、覆盖确认与 TOCTOU、取消/失败旧目标保护、路径与符号链接。SP-11 必须覆盖三平台外部拖入/拖出及限制披露。SFTP 相关功能为 M3/P0，不属于已实现的 M2 SSH MVP；原生拖出未验证通过的平台不得宣称该项交付完成。

## 首期固定入口边界（2026-10-15）

用户确认本轮只实现固定 SFTP Tab 与双栏未连接布局，实际文件浏览、端点连接和传输仍属 M3。该标签固定在首页之后、SSH 标签之前且不可关闭；侧栏模式固定在顶部。页面明确标记尚未开放，不产生 SSH/SFTP 会话、不列出伪文件、不提供无后端的上传下载操作。此布局不代表 M3-06 或任何 SFTP P0 已完成。

# SSH 连接、安全信任与认证

日期：2026-10-15。首期直连 SSH2 已按 russh 0.64.1 源码实现；未编译或连接真实主机。依赖公告与待验收项见 [实施状态](17-implementation-status.md)。

## 1. 连接配置

`SshProfile`：address、port（默认 22）、username、authPlan、terminalType（默认 xterm-256color）、初始 rows/cols、timeoutPolicy、keepAlivePolicy、algorithmPolicyRef。保存记录只包含公共或主机专属 credentialRef；私钥从凭据库读取，不在连接时引用原文件，不包含密码、私钥内容或 OTP。

address 输入不支持夹带端口或 shell 片段；UI 分别处理端口，允许域名/IPv4/IPv6，后端拒绝控制字符与嵌入 NUL。IPv6 展示时加括号，但 canonical key 不依赖字符串拼接猜测。DNS 用于定位网络目标，不能代替 SSH 主机身份校验。

## 2. 正常流程

```mermaid
sequenceDiagram
  participant UI as Vue 工作台
  participant App as Manager / Challenge Broker
  participant SSH as SSH Driver
  participant DB as HostTrustStore
  participant Remote as SSH Server
  UI->>App: open_session + 两个 Channel
  App-->>UI: Created 快照
  App->>SSH: 异步启动
  SSH->>Remote: DNS / TCP / SSH KEX
  Remote-->>SSH: 主机公钥与握手证明
  SSH->>DB: 读取该 endpoint 的信任记录
  alt 精确匹配
    DB-->>SSH: trusted
  else 未知
    SSH->>App: HostKeyChallenge
    App-->>UI: 指纹 / endpoint / 三种选择
    UI->>App: respond_host_key
    App-->>SSH: AllowOnce / TrustAndSave / Reject
  else 发生变更
    SSH-->>UI: HOST_KEY_CHANGED，阻止连接
  end
  SSH->>Remote: 用户认证（仅信任通过后）
  SSH->>Remote: open session / request PTY
  Remote-->>SSH: PTY success
  SSH->>Remote: request shell
  Remote-->>SSH: shell success
  SSH-->>UI: Connected + 数据流
```

上图为逻辑流程，不意味着 driver 直接引用 UI；实际通过 core 控制事件和 Tauri 桥接。引用凭据可在配置中存在，但秘密读取/提交应尽量延后至主机身份通过之后。

## 3. 主机密钥策略

### 首次信任

russh Handler 的 server key 检查接入 HostTrustStore。未知 endpoint 暂停握手，由独立控制 Channel 呈现：原输入 host、实际端口、key algorithm、SHA256 指纹、连接来源。提示用户通过可信渠道核验，不能声称首次弹窗本身已证明主机可信。

- Reject：默认选项，结束连接。
- AllowOnce：只限当前连接尝试；不写 SQLite，重连再次询问。
- TrustAndSave：验证 challenge 仍有效，在事务内持久化完整公钥及元数据，成功后继续。写失败不得静默变成永久信任；用户可以重新选择 AllowOnce。

Challenge 绑定 session、owner、key digest 和 endpoint；超时、取消、窗口关闭、阶段改变立即作废；只能成功消费一次。两个并发连接对同一 endpoint 的确认需事务内重查，出现冲突时按变更阻断处理。

### 身份与匹配

信任范围为应用内显式规范化后的 host + port，不自动按解析后的 IP 合并别名。规范化域名大小写、IPv6 表示，明确存储输入名与规范名；不自动推导通配符信任。

每个 endpoint 可保存多把经用户显式核验的完整公钥。匹配必须比较完整公钥类型和内容，不只比较 UI 指纹字符串；“发现一个新 algorithm 所以查不到同算法记录”不能当成新主机绕过现有 pin。endpoint 已存在信任但服务端本次 key 不属于集合，一律 HOST_KEY_CHANGED。

首期不实现主机证书 CA、`@cert-authority`、hashed known_hosts、自动轮换证明，也不自动读取/覆盖 `~/.ssh/known_hosts`。后续通过专用导入设计扩展，遇不支持项明确提示。

### 密钥变更

连接阶段直接阻断，展示历史与当前指纹。普通弹窗无“自动接受变更”快捷路径；用户须先通过带影响说明的信任管理界面撤销/替换，重新连接并核验。所有重连都重新检查，不能复用上次 AllowOnce。

## 4. 认证策略

`authPlan` 只保存一个主要认证方式及凭据引用，不保留旧版有序步骤兼容；不是无限尝试所有本地密钥。用户无需配置“密码 / 私钥之后继续交互认证（OTP）”。主机身份通过后，后端先发送不携带秘密的 `none` 请求查询可用认证方法，再按服务端 `remaining_methods` 与 `partial_success` 协商。服务器若允许 `none`，直接完成认证。

| 方式 | 首期行为 | 数据边界 |
| --- | --- | --- |
| 密码 | 读取公共/主机专属 OS 密码，未设置则 challenge 询问 | 不写入主机配置/SQLite/Pinia 持久化 |
| 私钥 | OpenSSH Ed25519 与 RSA-SHA2 优先；从 privateKey 凭据读取正文及可选解密密码 | 前端仅见文件名/公钥指纹，完整私钥不回传 |
| 私钥解密密码 | 可随私钥凭据保存；缺失/错误时通过 masked challenge 本次补充 | 不再独立建凭据；本次补充不自动保存 |
| Keyboard-interactive | 显示 banner/name/instructions 与每轮 prompts，按 echo 标记输入 | echo=false 必须掩码；OTP 永不持久化 |
| partial success | 保留已完成因素，继续服务器要求的下一步 | 不把多因素要求误报成错误密码 |

认证由服务器与配置共同决定；密码 auth 和 Keyboard-interactive 是不同协议步骤。密码 / 私钥部分成功且服务器要求 Keyboard-interactive 时立即进入服务端挑战；仅支持 Keyboard-interactive 的主机不会先索要无效的密码或私钥口令。主认证失败后如服务器仍提供交互认证，也允许自动进入 PAM 表单。配置不含 Keyboard-interactive 步骤，后端不会重复启动已处理的挑战组。保存的密码不会自动填入服务端任意提示；OTP 不持久化。常见 password+OTP 需要真实服务器验证；文档有方法并不表示应用已兼容任意 PAM/MFA。

首期密码提交共享最多 3 次预算，私钥口令最多 3 次重新输入；交互认证最多启动 3 次，所有交互尝试共享最多 8 轮 challenge（均为初始上限），每轮设置期限；服务端提示数量、单条长度和总字节数均需限额。拒绝复用旧 challenge。不要将完整 server banner 当作可信富文本，所有提示按普通文本显示。

M1 可以仅实现单次密码验证纵向切片；M2 才要求私钥凭据、可选解密密码与多轮交互完整。Agent、Pageant、证书、FIDO、安全密钥、PPK/PEM 全格式与 GSSAPI 不纳入 M2 承诺；不支持时明确说明，不默认调用系统 ssh 回退。

## 5. PTY、Shell 与数据

认证成功后开 session channel，依次请求 `TERM=xterm-256color` PTY 与交互 Shell，并等待相应协议成功响应。有些方法返回值只说明请求发送成功，具体接受状态应以选定 release 的回复机制处理。

初始尺寸来自已挂载且有效的 xterm；可暂用 80×24 兜底，随后以可见实测尺寸更新。resize 校验 rows/cols 为 1–1000 的整数，去重并合并，只发 window-change，不重建会话。不要默认在远端执行 locale 设置命令；UTF-8 显示与远端 locale 不一致时提示用户调整。

输入作为 bytes 保序写入。TTY 模式的 Ctrl+C 发送终端控制字节，由远端 PTY 行规处理；不得把用户键盘行为翻译成本地进程终止。处理 Data/ExtendedData、EOF、ExitStatus、ExitSignal、Close；PTY 输出中 stderr 合并展示，底层仍保留扩展类型供诊断，不尝试从字节流还原标准输出边界。

普通终端复制选区不应向远端写数据；中文输入法组合阶段不提前发送不完整字符；粘贴使用终端支持的 bracketed paste 与输入队列。

## 6. 超时与保活初始策略

| 阶段 | 初始值（待测） | 处理 |
| --- | --- | --- |
| DNS | 10 秒 | 独立取消/错误阶段 |
| TCP | 总预算 15 秒 | 多地址尝试共享预算，含 IPv6/IPv4 回退 |
| SSH 握手 | 20 秒 | 与等待用户确认分开计时 |
| 主机信任 | 120 秒 | 超时拒绝，提示重新连接 |
| 认证交互 | 每轮 120 秒，总交互预算 10 分钟 | 防无限 challenge；用户可随时取消 |
| 非交互认证请求 | 30 秒 | 明确失败，避免重复提交秘密 |
| PTY / Shell | 每步 10 秒 | 请求拒绝与超时分开 |
| KeepAlive | 30 秒空闲探测，连续 3 次无回复判定 | 实际语义按选定 russh release 验证 |
| 关闭 | 3 秒资源回收预算 | 到期释放 transport 并报告清理结果 |

握手等待信任时不继续消耗 20 秒网络超时；要么拆分阶段定时器，要么显式暂停并保留 120 秒挑战期限。KeepAlive 不等于“用户 N 秒没按键就退出”；默认不设用户活动空闲断线。系统睡眠唤醒后若存活性不明，呈现检测状态，失败后终止旧 session，等待用户重连。

## 7. 算法与兼容性

默认使用锁定 release 的安全算法列表，在设计审核时记录实际协商策略，优先现代 KEX/AEAD、Ed25519/RSA-SHA2；禁用 DSA、ssh-rsa/SHA-1 签名及弱 KEX 的自动回退。RSA 密钥格式不等于使用 ssh-rsa SHA-1 签名。

库“支持算法”与项目“启用算法”分别记录。老旧交换机若需要弱算法，后续通过单主机显式策略开启，并显示影响；不得全局放宽。首期不自行设计密码算法，不承诺因使用 Rust 就没有安全漏洞。

## 8. 重连与未来专业功能

重连建立新的后端连接：新 requestId/sessionId、重新握手、检查主机密钥、重新认证；不重放输入、不重新执行 snippet。**同一 Tab、terminalId、xterm 实例和滚屏缓冲必须保留**，新输出在原终端末尾继续追加，不创建替代视图或清空缓冲。断线、首次连接失败、重连失败均保留 Tab 和历史滚动/复制/搜索能力；没有历史的首次失败也保留空终端与可查看的错误信息。

新旧连接之间通过可信本地状态消息标识时间与重连结果，不把错误当作新远端输出；旧流排空、代次隔离及终端模式边界见 IPC/交互文档。保留显示历史不代表恢复原远端 Shell；远端任务恢复仍需 tmux/screen 等远端能力。

ProxyJump 后续通过明确 hop 列表，为每一跳独立校验密钥与认证；不接受任意 ProxyCommand 自动执行。SFTP/隧道走独立服务与 connection 租约，见架构文档，不向通用终端加入 listFiles/startTunnel 等方法。

## 首期源码与差异（2026-10-15）

实现分别见 terminal-ssh 的 connection/host_key/auth/driver 模块。已实现密码重试、加密 OpenSSH 私钥口令重试和最多 8 轮 Keyboard-interactive。2026-10-15 重构后，后端依据服务端可用方法与部分成功自动续接 OTP / PAM，使用单一主要 authPlan，界面仅提供密码/公钥，既不提供独立交互认证选项，也不要求预设追加交互认证。公共密码/私钥在凭据页保存；主机直接密码非空自动保存为专属项，空值连接时询问；认证弹窗只用于本次。

固定配置使用 256 KiB SSH window、32 KiB maximum packet、16 channel 消息容量、30 秒 KeepAlive 与 max=3。russh 接收任意包会重置保活计数，且超出 max 才断开，不能称“精确 90 秒”。库内部 channel 满时可能影响协议调度，SP-05 仍是未通过门槛。独立 socket shutdown 确保取消不依赖输出 ACK。

已从默认 host-key 算法排除 RSA/SHA-1；RSA 私钥仅选择服务器明确支持的 SHA-2 签名。此算法选择不修复 RSA 实现侧信道公告，详见 [实施状态的公告记录](17-implementation-status.md#4-依赖与公告核查)。没有真实握手/认证/PTY 验证记录。

本次静态核查：对照本机已下载的 russh 0.64.1 源码核对 `authenticate_none`、`AuthResult`、`MethodKind` 与交互结果 API；对修改文件执行 rustfmt 格式检查与差异检查。未执行编译、Rust 测试、SSH 连接或运行验收。后续需验证公共/专属密码、加密/无密码私钥、密码+OTP、私钥+OTP、仅 PAM、错误密码/OTP 重试与取消/超时。

本轮按用户要求不考虑旧数据库与认证配置兼容：删除静态 KeyboardInteractive 枚举和独立 passphrase 凭据引用。连接时自动 KI 协商保持有效。

# SSH 连接、安全信任与认证

更新：2026-10-21。直连 SSH2、PTY Shell/exec 和系统识别已有源码；认证互通、流控及三平台完整验收尚未完成。统一生命周期见 [会话模型](04-session-model.md)，依赖风险见 [产品需求](01-product-requirements.md)。

## 1. 连接配置

权威类型为 [SshConfig](../crates/terminal-core/src/types.rs)：`configVersion`、`address`、`port`、`username`、`authPlan`、`startup`。保存主机连接只传 ProfileId，由 Rust 重读配置及加密凭据；私钥使用保险库凭据，不读取本地私钥文件。主机配置保存凭据引用，不保存秘密正文或 OTP。

地址支持域名、IPv4、无方括号 IPv6，端口单独填写，默认 22。后端拒绝控制字符、空白、嵌入端口等非法形式。DNS 只定位目标，不替代主机身份验证。

`startup` 默认为 `{type:"shell"}`，或 `{type:"exec",command}`。命令须非空白、无 NUL、最多 8192 UTF-8 字节；原文发送给服务端，不拆参数、不补引号或拼接包装命令。

## 2. 连接顺序

DNS/TCP → SSH 握手与主机密钥核验 → 用户认证 → session channel → PTY → Shell 或 exec → Connected。读取和提交认证秘密发生在主机身份通过后；PTY、Shell、exec 分别等待协议确认，拒绝或超时报错，不回退其他启动方式。

文件和转发使用专用工厂，只复用身份、认证及传输，不执行保存主机的启动命令，见 [SFTP](16-sftp-workspace.md)、[端口转发](21-port-forwarding.md)。

## 3. 主机密钥策略

信任按规范化的 host + port 匹配完整公钥类型与内容；不按 DNS 解析结果合并别名，不自动推导通配符。未知密钥须核验 SHA256 指纹后明确选择 Reject、AllowOnce 或 TrustAndSave。AllowOnce 只限当前连接；永久保存成功才继续，写失败不视为已保存。

挑战绑定会话、owner、endpoint、公钥与期限，只能消费一次；取消或超时即作废。并发保存事务内重查，冲突按密钥变更阻断。

已信任 endpoint 出现不同公钥一律 `HOST_KEY_CHANGED`，更换算法不能绕过 pin。须在信任管理中显式撤销旧记录后重新连接核验；每次重连均重查。当前不读取/覆盖系统 known_hosts，不支持主机 CA、hashed known_hosts 或自动轮换证明。

## 4. 认证策略

`authPlan` 恰好一个主要方式：密码或私钥。先用无秘密的 `none` 请求查询服务端方法，再按 `remaining_methods` / `partial_success` 协商；服务端允许 none 时直接成功。不会遍历本地所有密钥或调用系统 ssh 回退。

主机新建和快速连接表单按认证方式筛选就绪凭据，并可直接打开对应类型的凭据创建弹窗。公钥认证必须选择私钥凭据；新建私钥凭据通过多行输入框粘贴完整私钥正文，并可填写解密密码，不使用文件选择器。凭据输入与公钥查看对齐当前固定 SSH 库支持的 OpenSSH、PEM（RSA、PKCS#8、EC 标记）及 PuTTY PPK 格式，具体算法和加密方式以该 release 解析器为准，不承诺全格式兼容。凭据列表提供公钥查看与复制：OpenSSH 读取封装中附带的公钥，无需解密口令；其余格式复用认证解析器，必要时使用保存的口令，或在查看弹窗临时输入后重试，临时口令不持久化。公钥展示不证明远端认证可用。密码认证的空凭据引用不使用共享凭据，保存主机时可填写专属密码，未填写时仅在连接期间询问。

- 密码从保险库凭据读取；未保存或失败时请求本次输入，补充输入不自动保存。
- 私钥正文与可选解密密码从加密凭据读取，缺失或错误的解密密码可本次补充。RSA 须确认 RSA-SHA2 支持，不回退 SHA-1。
- 服务端要求 Keyboard-interactive/PAM/OTP 时动态展示提示，遵守 echo 标记；OTP 不持久化。部分成功后继续下一因素，仅 PAM 服务不会先索要无效的密码或私钥口令。

保存密码仅可自动填入单个非回显、精确为 Password/密码（忽略英文大小写及末尾冒号）的提示，每连接最多一次；已提交过密码协议后不再自动填入。OTP、复合或未知提示均由用户输入。零提示且名称/说明空白的轮次直接回复空响应，其余通知仍展示；所有轮次均受限额约束。

密码提交最多 3 次，私钥口令最多 3 次补充，交互认证最多启动 3 次且共享 8 轮挑战；提示数量、长度和总字节受限。服务器提示按普通文本显示。Agent/Pageant、证书、FIDO/GSSAPI 与全格式私钥兼容尚未实现。

## 5. PTY、Shell 与命令

两种启动模式均请求 `xterm-256color` PTY，支持输入和 resize；exec 只发送配置命令，不先启动 Shell 或模拟键盘输入。请求确认期限不限制程序运行时长。无 PTY、自定义 cwd/env 和自动重启未实现。

确认期间的输出及退出消息有界保留，接受后进入统一输出/ACK 流程；正常结束先提交尾输出，再发布退出码/信号。命令退出保留终端，不转入交互 Shell；重新运行须用户显式建立新连接。命令模式不参与输入广播。

尺寸限 1–1000，初始可用 80×24 兜底，随后发送最新有效尺寸；不重建会话。Ctrl+C 作为终端字节由远端 PTY 处理；不自动执行 locale 配置命令。PTY 的普通与扩展输出合并展示，不从字节流推断标准输出边界。

## 6. 超时与保活

当前固定值：DNS 10 秒，TCP 总预算 15 秒，握手网络时间 20 秒，信任及每轮挑战 120 秒，认证总预算 10 分钟，非交互网络请求 30 秒，PTY/Shell/exec 每步确认 10 秒。握手等待信任时暂停网络计时，取消独立生效。

KeepAlive 间隔 30 秒、max=3，不设用户活动空闲断线；russh 收包重置计数且超出 max 才断开，不能解释为精确 90 秒。专门睡眠恢复尚未实现。

## 7. 算法与资源

使用锁定 russh release 的算法列表，显式移除主机密钥 `ssh-rsa/SHA-1`，RSA 用户认证只允许 SHA-2；不自动启用弱算法。每 SSH channel 配置 256 KiB 窗口、32 KiB 最大包和 16 条消息缓冲。取消通过底层 transport shutdown 生效，不依赖输出 ACK；库内部队列满时的公平性仍待实测。

## 8. 重连与扩展边界

重连重新握手、核验密钥和认证，沿用原 TerminalId/xterm/滚屏缓冲，清除旧输入且不自动重放命令。保存主机的显式重新运行读取最新配置，临时连接沿用打开配置；既有会话不受后续主机编辑影响。

ProxyJump、ProxyCommand、Agent、SSH config 导入与通用连接租约尚未实现。保留终端显示历史不代表恢复远端进程；远端任务恢复依赖 tmux/screen 等能力。

## 9. 源码与验证边界

实现入口：[connection](../crates/terminal-ssh/src/connection.rs)、[host_key](../crates/terminal-ssh/src/host_key.rs)、[auth](../crates/terminal-ssh/src/auth.rs)、[driver](../crates/terminal-ssh/src/driver.rs)。真实密码/私钥/OTP/PAM、取消/超时、快速退出尾输出、全屏程序与资源回收仍待完整运行验收；静态核对 API 不等于服务端兼容性通过。

## 10. 一次性主机系统识别

仅保存的交互 Shell SSH 终端生效，`detectSshSystem` 默认开启；临时连接、命令模式、Telnet、文件及转发不检测。PTY/Shell 建立后事务 claim 一次机会：须尚未尝试且仍为默认 SSH 图标。关闭开关或命令模式跳过不消耗机会；claim 后失败、超时或中断也不重试，修改目标不重置标记。

先读取受界的 SSH 服务端标识，必要时通过独立 exec 执行 [固定只读命令](../resources/ssh-os-detection.json)。所有探测共用 5 秒、32 KiB 输出预算，同一时刻仅一个探测 channel；只解析 stdout，不 source 远端文件或将返回文本拼入命令，不写 PTY、不进入广播。失败不影响终端状态，探测可能被服务器审计。

结果仅用于白名单图标；提交时重查令牌、目标和默认图标，避免覆盖用户修改或复活已删主机。目标或图标变化作废旧令牌，不清除尝试标记；复制主机独立计算尝试状态。设置只提供开关及用途说明，不开放脚本编辑或任意 exec IPC。实现见 [os_detection](../crates/terminal-ssh/src/os_detection.rs)，兼容性待运行验收。

# 安全与隐私设计

日期：2026-10-15。M1/M2 安全约束已开始在模块化代码中落实，当前为“实现待验证”；本轮没有编译、启动、SSH 连接或三平台安全验证，不代表安全门槛已通过。

## 1. 威胁模型与边界

| 威胁/输入 | 主要措施 | 限制 |
| --- | --- | --- |
| 网络中间人/冒充服务器 | SSH 握手证明 + 应用主机公钥 pin/首次核验/变更阻断 | 首次 AllowOnce/TrustAndSave 仍依赖用户核验 |
| 恶意远端终端输出 | xterm 字节渲染、无 HTML 注入、禁用敏感 OSC 动作 | 需要持续更新终端解析依赖 |
| 前端 XSS/第三方内容 | 打包本地资源、CSP、最小 IPC、禁止远端导航 | 被攻陷的主窗口仍可能调用其合法权限，不能宣称 ACL 完全隔离主窗口 |
| 跨窗口资源访问 | 调用者身份、session/challenge/fileRef owner 检查 | 随机 ID 不能替代授权 |
| 设备上其他普通用户 | 系统凭据库、应用数据权限、私钥读取边界 | 同用户恶意进程/root 不属于可完全防御范围 |
| 大量输出/长认证提示 | 字节限额、信用流控、队列/滚屏/提示长度上限 | 需同时核验协议库内部缓冲 |
| 日志/备份泄露 | 默认不记内容、秘密脱敏、用户主动导出 | 元数据本身仍可能敏感 |

不实现自定义 SSH 加密，不把项目描述为经过安全审计的 Vault。未来云同步、Agent forwarding、隧道监听各自需要新威胁分析。

## 2. Tauri 权限落地要求

模板 greet 与宽泛默认权限已替换：[`build.rs`](../src-tauri/build.rs) 使用 `AppManifest::commands`，命令名单见 [`command_names.rs`](../src-tauri/src/command_names.rs)，主桌面窗口 capability 见 [`default.json`](../src-tauri/capabilities/default.json)。所有数据/终端用例先核验调用窗口，session/challenge 再由 core 核验关联；凭据与私钥引用当前只供 `main` 使用，不提供多窗口共享授权。生成的权限清单、实际拒绝行为仍须在授权构建/运行后验证。

1. 按 [官方 capability 文档](https://v2.tauri.app/security/capabilities/) 把应用命令登记到 `tauri_build::AppManifest::commands`，生成/声明 allow/deny 权限，仅授予主窗口实际需要的命令。应用自定义命令默认可由应用窗口调用，不能只写 capability 后以为自动拒绝。
2. capabilities 只匹配已知 main 窗口，限定桌面平台，不开放远程 origin 的 IPC；移除无需使用的默认插件权限。
3. 命令接收 Tauri 注入的 WebView/Window 上下文，构造 owner；所有 session/凭据引用/文件授权/challenge 都在服务端验证资源关联。
4. 不开放 shell plugin 任意执行、前端任意网络 socket、任意 fs 读写、原始 SQL、读取所有凭据等广义命令。
5. 默认只加载本地打包前端。拦截 WebView 远端导航与新窗口；帮助链接由受控 opener 在外部浏览器打开。

ACL 是窗口能调用哪些命令；业务所有权决定命令能操作哪个对象，两层都需要。

## 3. CSP 与依赖资源

当前 [`tauri.conf.json`](../src-tauri/tauri.conf.json) 已配置发布 CSP 与开发 `devCsp`，覆盖 script-src/style-src/connect-src/img-src/font-src/object-src/base-uri/frame-src，阻止远端脚本、eval 与嵌入网页；保留 Tauri IPC 实际所需的 origin/protocol，按目标平台和锁定版本核实。xterm 的样式与 WebGL 行为需要测试，不能套 `default-src 'self'` 后声称三平台可运行。

不从 CDN 加载 Vue、字体或终端脚本。开发 HMR 的 localhost/WebSocket 仅限开发配置，不能把它们宽泛放进发布 CSP。尽量用 nonce/hash 或精确资源规则；确需样式例外时记录原因和最小范围。当前 `style-src` 的 `unsafe-inline` 用于 Vue/Naive UI CSS-in-JS 与 xterm 运行时样式；不允许脚本 eval、远端脚本与 frame。该样式例外并不等于已证明 XSS 防护完整。导航白名单、新窗口拒绝与 `open_js_links_on_click(false)` 见 [`lib.rs`](../src-tauri/src/lib.rs)；外链经 [`commands/native.rs`](../src-tauri/src/commands/native.rs) 校验 http/https、主机、控制字符/空白、用户信息与长度后才交系统浏览器。SP-02/SP-07 与三平台验证后才能冻结最终策略。

## 4. 主机密钥与算法

必须在认证秘密提交前完成 server key 信任；未知默认拒绝，明确允许才继续，变更密钥阻断。详见 [SSH 设计](05-ssh-design.md)。保存完整公钥，不只保存可混淆展示文本；端口、别名与算法变化不能绕过 pin。

当前 SSH 适配固定 `russh=0.64.1`，在 [`connection.rs`](../crates/terminal-ssh/src/connection.rs) 从主机签名偏好移除 RSA/SHA-1；[`auth.rs`](../crates/terminal-ssh/src/auth.rs) 的 RSA 私钥认证要求服务端声明 SHA-256/512，不自动回退 SHA-1。其他算法使用该 release 默认配置，仍需完整安全公告/算法审查。证书主机密钥当前显式不支持，不能宣称兼容所有 SSH 服务端。

[`host_key.rs`](../crates/terminal-ssh/src/host_key.rs) 在身份通过前不读取认证凭据；[`terminal-storage/src/metadata.rs`](../crates/terminal-storage/src/metadata.rs) 比较 endpoint、算法与完整公钥，保存时事务内重查，禁止并发确认覆盖已存在的不同密钥。普通连接弹窗不能替换旧 pin；必须先在信任管理中明确撤销再重新核验。持久信任目前没有独立“为相同 endpoint 新增不同 key”的用例。

## 5. 凭据与内存

- 公共秘密由用户明确创建；主机编辑直接输入非空密码即授权保存该主机专属项，空值则清除已有专属密码；应用只提供写入/删除/使用引用的业务操作，不提供读取明文给 UI 的接口。
- 前端密码框不可避免短暂持有输入和 IPC 序列化副本；提交/取消后清空，不写 Pinia、localStorage、错误追踪或持久缓存。不能承诺 JS 字符串可靠擦除。
- Rust 当前使用 `zeroize::Zeroizing` 包装认证答案、口令、私钥文本等短期秘密，禁止派生 Debug/Serialize 泄露；库调用产生的副本及 OS 内存仍需认识限制，不承诺绝对清零。
- 私钥导入在 Rust 内限大小读取，在 OS 凭据库与其可选解密密码共同保存；连接时读取材料并解析，短期口令使用后释放；不写临时明文私钥文件。
- OS 凭据库锁定、服务缺失或用户拒绝时，保存必须明确失败；主机密码可留空并在连接时本次输入；永不自动使用 mock/sample、明文配置或自制弱加密回退。
- 凭据读写使用受控工作器，禁止阻塞 Tokio worker；同一 credential 操作串行，UI 线程不等待系统弹窗阻塞。

## 6. 终端输出安全

所有远端字节仅进入 xterm；连接标题、banner、认证提示、历史与文件名按文本显示，禁止 v-html/innerHTML。控制字符在 UI 元数据中清理并限长，但不能为了清理标题而修改原始终端 bytes。

默认禁用 OSC52 剪贴板读写；不实现远端触发命令、自动打开 URL、自动下载或自动写文件。链接默认仅由用户 Cmd/Ctrl+点击（可配置终端手势）触发，经协议白名单 `https`/`http` 后交受控 opener；拒绝 `file:`、`javascript:`、任意自定义 scheme，防止换行/混淆链接。终端标题变化只改变受限标签标题，不能改变已保存的主机身份或信任提示。

多行粘贴默认确认，可由用户配置；不自动 trim、换行替换或添加 Enter。远端的 bracketed-paste 状态和交互行为需验证。后续日志导出若实现，内容仍可能包含终端转义序列，文本查看器应安全呈现；当前没有会话日志导出。

## 7. 文件与路径

私钥选择在原生侧建立授权引用；后端限定用途与文件类型/大小，不提供通用读文件接口。检查实际打开对象而不仅是字符串路径，关注符号链接和选择后替换的 TOCTOU；无法完全消除时重新显示文件指纹并要求用户核验，而非声称路径绝对安全。

M3 SFTP 的本地根授权与私钥授权分开。原生选择/drop 签发 owner-bound rootGrant/entryRef，检查实际打开对象、父目录和符号链接，不信任前端路径、远端文件名中的 ../绝对路径/控制字符；下载不得逃出授权目录。远端操作同样绑定端点/真实名称，文件名仅按文本显示，不能拼 Shell。

上传/下载/远端中转默认询问覆盖；预检与发布前复核、exclusive create/临时项/安全 rename 按能力采用。不把 stat+rename 宣称通用原子条件覆盖，缺少保证时不自动删旧目标。批量冲突同意限定具体任务，旧 challenge/目标变更不能复用；元数据修改按权限/能力处理，递归删除另行确认。完整策略见 [SFTP 文档](16-sftp-workspace.md)。

Remote A→B 同时校验两端所有权、身份/信任和源/目标快照，默认有界内存中转不落盘；文件正文不进 UI/日志。原生拖出需要受限 staging 配额/权限/到期清理，只删本任务确切登记文件，尊重 OS 读取时机与冲突取消；不能以虚拟路径绕过授权。隧道默认只绑定 loopback，监听外网需显式选择；ProxyCommand 导入默认禁用执行。M2 不提前暴露 M3 文件接口。

## 8. 观测、日志与诊断

当前没有接入持续应用日志文件、会话正文日志或诊断导出；连接历史只记录显示名、endpoint、起止时间、终态原因/错误码，属于敏感元数据，不是终端内容。后续应用日志只允许 requestId、sessionId、阶段、耗时、错误码、队列水位，不记录密码、口令、OTP、私钥、完整输入和输出；主机地址/用户须按隐私策略处理，日志轮转与限额（初始 5×5 MiB）仍待实施。

“终端会话日志”是 M3 单独 opt-in 功能，可能包含秘密输出，不能因为应用日志脱敏就声称会话日志安全。诊断导出先展示内容类别并移除秘密与文件绝对路径；默认无遥测/崩溃上传，任何外发另经用户选择。

## 9. 发布前安全门槛

必须验证错误密钥拒绝、跨窗口/旧 challenge 拒绝、输出洪泛下限额、DB/日志无秘密、CSP/导航限制、依赖安全公告与许可证。安装包签名、公证、更新签名、依赖扫描安排在授权的验证/发布流程；此文不授权自动运行构建或新增会在推送时编译的 CI。

## 10. 快捷键与配色导入边界

快捷键导入只能映射已注册 commandId/合法上下文/白名单按键，拒绝脚本、shell、自动远端输入与任意原生动作；系统后台热键另经 opt-in，不默认抢占 OS。自定义链接手势仍只开 http/https，不能扩大 opener 权限。录制状态不向远端提交输入，也不执行所录动作。

配色导入限制大小、数量、深度与颜色字段，校验 schema/revision，预览后持久化；拒绝 CSS 表达式、外部 URL、脚本、plist 外部实体与压缩包解压路径。颜色允许低对比度由用户决定但提示风险，不把主题文件当作可信代码或系统配置。详见 [14](14-shortcuts.md) 与 [15](15-terminal-themes.md)。

## 11. 本轮安全实现、差异与验证边界

- [`terminal-security/src/vault.rs`](../crates/terminal-security/src/vault.rs)：`keyring=3.6.3` 使用 macOS Keychain、Windows Credential Store、Linux Secret Service；操作在 `spawn_blocking` 内串行执行，不阻塞 Tokio worker。写入先 pending、OS 成功后 ready；删除先检查共享引用再 deleting、删除 OS 项后删元数据。服务锁定/缺失/拒绝不使用 mock、SQLite 密码或自制加密替代。用户确认 pending 恢复；启动仅定向协调本 DB 的 deleting 项。实际系统弹窗、失败状态与重启补偿未验证。
- [`terminal-security/src/private_key.rs`](../crates/terminal-security/src/private_key.rs)：只接受后端原生选择器登记的私钥引用，读取限 1 MiB/OpenSSH 文本，不向前端返回正文；Unix 拒绝符号链接、非当前 uid 与其他用户有权限的文件，使用 `O_NOFOLLOW` 并核验打开对象的设备/inode。注册时和导入时共用 [`terminal-storage/src/key_files.rs`](../crates/terminal-storage/src/key_files.rs) 的安全读取边界；SQLite 只保存文件字节 SHA-256 摘要，导入引用前比对，内容变更即拒绝并要求重新选择。该摘要不充当 SSH 主机密钥指纹，且不序列化给前端。哈希借用 Zeroizing 字节缓冲，不另建裸私钥副本。Windows 的文件对象竞态/ACL 校验仍需完善；读取中的并发原地写入、重新选择意图与平台行为尚未运行验证，不应声称已消除全部 TOCTOU。
- [`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs)：单实例插件先于数据库初始化，Unix 应用数据目录 0700；[`terminal-storage/src/lib.rs`](../crates/terminal-storage/src/lib.rs) 对 DB/备份设 0600，未来 schema 先只读检查。Windows 数据目录/文件 ACL 使用系统继承，仍需验收；SQLite 不加密，OS 凭据库不保护其中主机名、用户名、私钥路径和历史摘要。
- [`src-tauri/src/settings.rs`](../src-tauri/src/settings.rs)：应用设置、快捷键、`customSchemes` 与 `hostSchemes` 都在 `settings.key=app` JSON 中，校验版本、长度、颜色字段、注册命令与方案引用，不是独立配色表或主机终端偏好表。没有脚本、任意原生动作或 shell 执行型导入。设置完整替换尚无 revision，后续多视图并发需补齐。详见 [存储实现差异](07-storage.md)。
- [`terminal-core/src/stream.rs`](../crates/terminal-core/src/stream.rs) 与 [`manager.rs`](../crates/terminal-core/src/manager.rs)：字节/消息双重队列预算、合法帧端点 ACK、独立取消、有限输入重试缓存；[`terminal-ssh/src/driver.rs`](../crates/terminal-ssh/src/driver.rs) 在输出阻塞时保留输入/resize/取消分支。xterm 解析完成后确认、旧 session 隔离与终态 finalOffset 均为实现，内存水位/洪泛/远端窗口公平性仍未测量。

验证记录（2026-10-15）：下载固定 release 源码核对 SQLx、keyring、Tauri、dialog/opener/single-instance 的所用 API/features；Rust 格式检查、存储/安全 Cargo TOML 解析与源码空白检查通过。编译、运行、真实凭据库、SSH、恶意远端输出、CSP/导航拦截、三平台 IME/剪贴板/权限、崩溃与迁移恢复均未执行。传递依赖 MSRV、完整许可证集合与最新安全公告仍待复核；不得把“源码已核对”写成依赖安全审计或发布门槛已通过。

内置字体边界（2026-10-15）：12个家族作为只读前端资源随包分发，CSP font-src 保持 self/data，不增加远端字体域名。保存字体名称走 Rust 白名单，不允许 CSS URL 或任意字体列表；旧字符串读取只归一返回值。来源、许可、原文件 SHA-256 保留于 public/fonts，静态校验不能替代 WebView 字体解析器和三平台运行验证。

## 工作区广播输入边界（2026-10-15）

广播需用户手动开启，默认关闭/不持久化，仅当前Workspace聚焦且已连接终端的用户输入分发到该组已连接成员，成员拖入/拖出/重组/关闭时自动关闭。源自动设备/光标/焦点应答不得跨连接，认证/信任弹窗、鼠标点击/拖动不广播。固定xterm 6.0.0内部onUserInput标记由独立只读订阅适配，缺失则拒绝开启广播，升级需专项复核；不采用仅靠聚焦状态判定全部onData为用户输入。

粘贴确认显示目标数，保留用户多行确认设置；等待期间组/开关/源会话变化取消，目标换代/断线不补发。每目标输入容量/序号/失败策略独立，禁止记录输入、自动重试或将未确认输入移交新会话；已有owner/ACL/命令校验继续适用，无新增秘密或系统权限。

OSC标题实现（2026-10-15）：remoteTitle仅作前端内存中的展示后缀，控制字符/双向格式字符清理、限160码点，Vue文本插值显示；不改写主机身份或信任提示，不写数据库/连接历史/日志，不据此执行动作。远端可任意设置该值，不应视为经验证的当前命令。

# 安全与隐私设计

日期：2026-10-16。M1/M2 安全约束已开始在模块化代码中落实，当前为“实现待验证”；尚无当前版本完整编译、SSH 连接或三平台安全验收，不代表安全门槛已通过。

## 1. 威胁模型与边界

| 威胁/输入             | 主要措施                                          | 限制                                                            |
| --------------------- | ------------------------------------------------- | --------------------------------------------------------------- |
| 网络中间人/冒充服务器 | SSH 握手证明 + 应用主机公钥 pin/首次核验/变更阻断 | 首次 AllowOnce/TrustAndSave 仍依赖用户核验                      |
| 恶意远端终端输出      | xterm 字节渲染、无 HTML 注入、禁用敏感 OSC 动作   | 需要持续更新终端解析依赖                                        |
| 前端 XSS/第三方内容   | 打包本地资源、CSP、最小 IPC、禁止远端导航         | 被攻陷的主窗口仍可能调用其合法权限，不能宣称 ACL 完全隔离主窗口 |
| 跨窗口资源访问        | 调用者身份、session/challenge/fileRef owner 检查  | 随机 ID 不能替代授权                                            |
| 设备上其他普通用户    | SQLCipher加密、主密钥钥匙串、应用数据权限与私钥读取边界            | 同用户恶意进程/root 不属于可完全防御范围                        |
| 大量输出/长认证提示   | 字节限额、信用流控、队列/滚屏/提示长度上限        | 需同时核验协议库内部缓冲                                        |
| 日志/备份泄露         | 默认不记内容、秘密脱敏、用户主动导出              | 元数据本身仍可能敏感                                            |

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

Keyboard-interactive 仅在主机信任通过后，对当前密码配置的单个非回显 Password/密码精确提示自动读取并发送保存密码一次；已尝试密码协议时不再自动发送。OTP、复合/未知/回显字段和重复提示均不自动填密码，不记录秘密。空白零输入轮自动回复空列表，非空通知仍由用户确认；两类响应共用提示长度/数量校验与八轮上限，避免自动响应绕过资源限制。

- 公共秘密由用户明确创建；主机编辑直接输入非空密码即授权保存该主机专属项，空值则清除已有专属密码；应用只提供写入/删除/使用引用的业务操作，不提供读取明文给 UI 的接口。
- 前端密码框不可避免短暂持有输入和 IPC 序列化副本；提交/取消后清空，不写 Pinia、localStorage、错误追踪或持久缓存。不能承诺 JS 字符串可靠擦除。
- Rust 当前使用 `zeroize::Zeroizing` 包装认证答案、口令、私钥文本等短期秘密，禁止派生 Debug/Serialize 泄露；库调用产生的副本及 OS 内存仍需认识限制，不承诺绝对清零。
- 私钥导入在 Rust 内限大小读取，在SQLCipher加密数据库中与其可选解密密码共同保存；连接时读取材料并解析，短期口令使用后释放；不写临时明文私钥文件。
- OS凭据库只缓存主密钥。自动读取失败时改为保险库密码派生和校验；成功后重存主密钥，缓存失败明确提示并保留本次解锁。错误密码、篡改或损坏不能覆盖数据库；不使用mock或明文回退。
- Argon2id和OS主密钥读写使用spawn_blocking；主密钥初始化专用门闩与凭据操作锁分开，不在会话全局锁或SQL事务内等待系统弹窗。主密钥派生参数固定64MiB/3轮/1并行度，密码不trim，不记录日志。

## 6. 终端输出安全

所有远端字节仅进入 xterm；连接标题、banner、认证提示、历史与文件名按文本显示，禁止 v-html/innerHTML。控制字符在 UI 元数据中清理并限长，但不能为了清理标题而修改原始终端 bytes。

默认禁用 OSC52 剪贴板读写；不实现远端触发命令、自动打开 URL、自动下载或自动写文件。链接默认仅由用户 Cmd/Ctrl+点击（可配置终端手势）触发，经协议白名单 `https`/`http` 后交受控 opener；拒绝 `file:`、`javascript:`、任意自定义 scheme，防止换行/混淆链接。终端标题变化只改变受限标签标题，不能改变已保存的主机身份或信任提示。

多行粘贴默认确认，可由用户配置；不自动 trim、换行替换或添加 Enter。远端的 bracketed-paste 状态和交互行为需验证。后续日志导出若实现，内容仍可能包含终端转义序列，文本查看器应安全呈现；当前没有会话日志导出。

## 7. 文件与路径

私钥选择在原生侧建立授权引用；后端限定用途与文件类型/大小，不提供通用读文件接口。检查实际打开对象而不仅是字符串路径，关注符号链接和选择后替换的 TOCTOU；无法完全消除时重新显示文件指纹并要求用户核验，而非声称路径绝对安全。

SFTP 的本地授权独立于私钥授权，仅来自原生选择器、系统拖入短期引用或 macOS 文件承诺目标 URL。前端不能提供任意本地路径。远端文件名拒绝点目录、分隔符和控制字符，只按文本显示，不拼接 Shell；下载须留在授权目标下。具体传输边界见第 13 节。

双栏远端互传逐次校验两端独立文件会话的窗口归属与连接状态，以64 KiB受界中转。独立本地浏览和通用连接租约仍为后续设计。隧道默认仅绑定 loopback，监听外网需显式选择；不自动执行 ProxyCommand。

## 8. 观测、日志与诊断

当前没有接入持续应用日志文件、会话正文日志或诊断导出；连接历史只记录显示名、endpoint、起止时间、终态原因/错误码，属于敏感元数据，不是终端内容。后续应用日志只允许 requestId、sessionId、阶段、耗时、错误码、队列水位，不记录密码、口令、OTP、私钥、完整输入和输出；主机地址/用户须按隐私策略处理，日志轮转与限额（初始 5×5 MiB）仍待实施。

“终端会话日志”是 M3 单独 opt-in 功能，可能包含秘密输出，不能因为应用日志脱敏就声称会话日志安全。诊断导出先展示内容类别并移除秘密与文件绝对路径；默认无遥测/崩溃上传，任何外发另经用户选择。

## 9. 发布前安全门槛

必须验证错误密钥拒绝、跨窗口/旧 challenge 拒绝、输出洪泛下限额、DB/日志无秘密、CSP/导航限制、依赖安全公告与许可证。安装包签名、公证、更新签名、依赖扫描安排在授权的验证/发布流程；此文不授权自动运行构建或新增会在推送时编译的 CI。

## 10. 快捷键与配色导入边界

快捷键导入只能映射已注册 commandId/合法上下文/白名单按键，拒绝脚本、shell、自动远端输入与任意原生动作；系统后台热键另经 opt-in，不默认抢占 OS。自定义链接手势仍只开 http/https，不能扩大 opener 权限。录制状态不向远端提交输入，也不执行所录动作。

配色导入限制大小、数量、深度与颜色字段，校验 schema/revision，预览后持久化；拒绝 CSS 表达式、外部 URL、脚本、plist 外部实体与压缩包解压路径。颜色允许低对比度由用户决定但提示风险，不把主题文件当作可信代码或系统配置。详见 [14](14-shortcuts.md) 与 [15](15-terminal-themes.md)。

## 11. 源码与验证边界

- [`terminal-security/src/master_key.rs`](../crates/terminal-security/src/master_key.rs) 管理唯一OS主密钥：Argon2id v19派生32字节主密钥，随机盐16字节，AES-256-GCM校验随机nonce12字节/AAD绑定格式身份、KDF和盐。启动每进程最多自动读取一次；密码恢复成功后重写主密钥，缓存失败仅警告。`vault.rs`只在加密DB读写凭据secret，DTO不返回秘密，未解锁拒绝所有业务IPC。
- [`terminal-security/src/private_key.rs`](../crates/terminal-security/src/private_key.rs)：只接受后端原生选择器登记的私钥引用，读取限 1 MiB/OpenSSH 文本，不向前端返回正文；Unix 拒绝符号链接、非当前 uid 与其他用户有权限的文件，使用 `O_NOFOLLOW` 并核验打开对象的设备/inode。注册时和导入时共用 [`terminal-storage/src/key_files.rs`](../crates/terminal-storage/src/key_files.rs) 的安全读取边界；SQLite 只保存文件字节 SHA-256 摘要，导入引用前比对，内容变更即拒绝并要求重新选择。该摘要不充当 SSH 主机密钥指纹，且不序列化给前端。哈希借用 Zeroizing 字节缓冲，不另建裸私钥副本。Windows 的文件对象竞态/ACL 校验仍需完善；读取中的并发原地写入、重新选择意图与平台行为尚未运行验证，不应声称已消除全部 TOCTOU。
- [`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) 保持单实例和Unix目录0700；数据库/解锁元数据创建为0600。SQLCipher 4 AES-256-CBC页面加密并以HMAC-SHA512校验，包含WAL；临时存储MEMORY，禁用SQL语句日志，拒绝普通SQLite降级。Windows ACL继承与原生构建仍需验证。
- [`src-tauri/src/settings.rs`](../src-tauri/src/settings.rs)：应用设置、快捷键、`customSchemes` 与 `hostSchemes` 都在 `settings.key=app` JSON 中，校验版本、长度、颜色字段、注册命令与方案引用，不是独立配色表或主机终端偏好表。没有脚本、任意原生动作或 shell 执行型导入。设置完整替换尚无 revision，后续多视图并发需补齐。详见 [存储实现差异](07-storage.md)。
- [`terminal-core/src/stream.rs`](../crates/terminal-core/src/stream.rs) 与 [`manager.rs`](../crates/terminal-core/src/manager.rs)：字节/消息双重队列预算、合法帧端点 ACK、独立取消、有限输入重试缓存；[`terminal-ssh/src/driver.rs`](../crates/terminal-ssh/src/driver.rs) 在输出阻塞时保留输入/resize/取消分支。xterm 解析完成后确认、旧 session 隔离与终态 finalOffset 均为实现，内存水位/洪泛/远端窗口公平性仍未测量。

验证边界：下载固定 release 源码核对 SQLx、keyring、Tauri、dialog/opener/single-instance 的所用 API/features；Rust 格式检查、存储/安全 Cargo TOML 解析与源码空白检查通过。编译、运行、真实凭据库、SSH、恶意远端输出、CSP/导航拦截、三平台 IME/剪贴板/权限、崩溃与迁移恢复均未执行。传递依赖 MSRV、完整许可证集合与最新安全公告仍待复核；不得把“源码已核对”写成依赖安全审计或发布门槛已通过。

内置字体：12个家族作为只读前端资源随包分发，CSP font-src 保持 self/data，不增加远端字体域名。保存字体名称走 Rust 白名单，不允许 CSS URL 或任意字体列表；旧字符串读取只归一返回值。来源、许可、原文件 SHA-256 保留于 public/fonts，静态校验不能替代 WebView 字体解析器和三平台运行验证。

## 12. 广播与远端状态提示

广播需用户手动开启，默认关闭/不持久化，仅当前Workspace聚焦且已连接终端的用户输入分发到该组已连接成员，成员拖入/拖出/重组/关闭时自动关闭。源自动设备/光标/焦点应答不得跨连接，认证/信任弹窗、鼠标点击/拖动不广播。固定xterm 6.0.0内部onUserInput标记由独立只读订阅适配，缺失则拒绝开启广播，升级需专项复核；不采用仅靠聚焦状态判定全部onData为用户输入。

粘贴确认显示目标数，保留用户多行确认设置；等待期间组/开关/源会话变化取消，目标换代/断线不补发。每目标输入容量/序号/失败策略独立，禁止记录输入、自动重试或将未确认输入移交新会话；已有owner/ACL/命令校验继续适用，无新增秘密或系统权限。

OSC 标题：remoteTitle仅作前端内存中的展示后缀，控制字符/双向格式字符清理、限160码点，Vue文本插值显示；不改写主机身份或信任提示，不写数据库/连接历史/日志，不据此执行动作。远端可任意设置该值，不应视为经验证的当前命令。

动态关闭提醒：OSC 133 执行阶段与 OSC 0/2 标题都可由远端设置，只用于体验提示，不代表真实进程检测或安全证明。已连接且执行标记为真，或当前标题第一个冒号前缀不同于本次首次标题前缀时提醒；忽略冒号后目录，不持久化标题基准。不采集输入、命令正文或退出码历史，不自动执行任何远端动作。缺少相关输出的 Shell 可能漏报。

## 13. SFTP 浏览与传输

7 个 SFTP 命令均经 AppManifest/ACL、main owner 和 session 归属校验，侧栏复用终端已认证连接，双栏从保存主机独立认证；公开用例为浏览、上传下载与对拷。目录创建只用于上传合并和下载落盘内部流程。

链接元数据以 READLINK/STAT 查询，按 Vue 文本展示，不执行 Shell。显式导航允许服务器跟随目录链接，保留逻辑路径；展示时类型不是后续授权。传输仅接受普通文件和真实目录，递归不跟随源链接或特殊文件。仅支持安全 UTF-8 名称，不保证任意字节名。

上传同名普通文件逐项原生确认后直接写最终路径：LSTAT 检查、OPEN WRITE、FSTAT 复核普通文件、FSETSTAT size0 后写入；新文件 CREATE|EXCLUDE。同名目录仅合并，类型冲突拒绝，无删除目录或远端 hardlink 发布路径。失败/取消可能留下部分文件，覆盖不可回滚。SFTPv3 无通用 NOFOLLOW/条件写入，远端恶意行为及父目录并发替换仍有 TOCTOU 风险，不宣称原子或严格源快照。

单文件下载以独占临时项和本地 hardlink 拒绝覆盖发布，临时文件 Unix 权限 0600。清理只处理本任务确切创建的路径，失败返回残留位置，不通配符扫删。macOS 目录承诺仅在精确授权的新根创建，拒绝同名目标；最多 64 层/10000 项，失败或取消保留已完成内容，不递归删除根目录。源 Unix 文件打开后比对设备/inode，Windows 对象身份及父路径替换仍需加固。

正文仅 Rust 流动，不进 WebView/Pinia/SQLite。SFTP 包上限 256 KiB，列表最多 10000 项，数据块 64 KiB，全局最多 4 个任务，单浏览器 1 个操作。网络/磁盘/对话框不在全局锁内等待，调用层不无限预读；错误诊断不回显协议正文。

原生拖入引用 owner 绑定、一次消费、5 分钟有效，最多 16 引用/每批 1000 根路径；无消费者不读取文件。仅当前可见侧栏消费，上传到当前目录，不按行定位。macOS 拖出最多 100 根文件/目录，directories 只是受校验的类型提示，真实类型在 Finder 请求内容时复核；目标 URL 只留 Rust。原生会话期间隔离上传事件，取消阻断迟到兑现，内部落点不返回 Copy。Windows/Linux 拖出未实现，不以网页 DnD 伪造。

隐藏继续任务，侧栏随终端断开/关闭取消，双栏两端独立连接，任一端断开取消对拷。文件工厂不启动Shell/PTY，不向WebView提供SSH handle。对拷只接收两端sessionId/逻辑路径/安全名称，在Rust重新校验类型、源目录预扫描64层/10000项；同名文件确认覆盖、目录合并，取消不回滚。相同已知连接标识下用真实路径阻止自复制/复制到子树；不同主机别名、远端hardlink及并发路径替换无法由SFTPv3通用接口证明对象身份，不保证原子覆盖。无通用连接租约、持久任务或重启清理。原生线程/兑现生命周期、Finder、三平台拖入、并发替换、取消及依赖公告须继续运行/安全验收；当前静态核查不代表安全门槛通过。

窗口标题栏（2026-10-16）：仅 main 窗口增加 core:window 的 allow-start-dragging、allow-internal-toggle-maximize、allow-minimize、allow-toggle-maximize 权限，使用 Tauri 内置窗口命令；不新增自定义 Rust 命令或放宽远端资源范围。关闭沿用已有 allow-close 与会话/传输确认，组件通过 desktop 门面操作。

加密保险库威胁边界：静态文件丢失不直接暴露主机与凭据，但不防御已解锁进程被注入、内存转储、同用户恶意程序或root。SQLx连接选项及SQLCipher内部必须持有可用密钥，Zeroizing只尽早清理受控临时值，不能承诺库内部和JS副本完全清零。校验元数据不含密码/主密钥，但可供离线猜测，密码强度与Argon2id成本仍重要；GCM认证与SQLCipher页HMAC不提供整库旧快照回放检测。密码及OS主密钥同时丢失不可恢复。旧明文库不会自动加密或清理。

主机分组删除（2026-10-16）：前端“一并删除”必须显示范围并独立二次确认；Rust继续校验main窗口/保险库门禁和根组revision，整树删除与专属凭据deleting标记在同一事务。只按owner_profile_id清理被删除主机的专属项，不删除公共凭据，也不主动关闭已建立会话。默认删除仍为保留上移。主机拖拽只提交轻量HostDraft并省略password，不读取秘密；分组父级由Rust再次防环。没有新增通用SQL或批量凭据删除接口；交互和回归测试源码待运行验证。

## 本地终端边界（2026-10-16）

本地Shell以当前用户权限运行，只有解锁后的main窗口可通过现有open_session ACL启动；本地目标仅接受source=local，不从前端接收可执行路径、启动命令、环境或cwd，不自动执行终端输出。启动继承桌面应用环境及用户主目录，不将环境或正文写入数据库；历史只记录“本地终端”及local摘要。Unix初始进程组清理不覆盖主动脱离会话的进程，Windows完整进程树清理仍待完善，三平台运行未验收。

## 设置窗口授权

独立settings窗口仅加载本地应用资源并禁止新窗口导航，只有已解锁时才允许创建。settings-desktop capability按命令开放设置读写和用户触发JSON导出；这些命令使用settings_owner核验main/settings标签、本地URL和保险库状态。会话、主机及凭据管理等通用owner仍仅允许main，禁止通过放宽window_owner获取终端/文件权限。设置通知由Rust在提交后发送，正文不含凭据。窗口授权拒绝路径与三平台运行验证尚未执行。

主机信任及连接历史已移至主窗口保险箱，对应list_known_hosts/revoke_known_host/list_history/clear_history改用owner，只允许main且要求已解锁；settings capability不再授予这些命令。撤销信任和清空历史继续先由UI确认，主机/凭据及活动会话不会因页面迁移改变。

## SSH 系统识别边界

系统识别仅在保存SSH终端已完成主机信任、认证和PTY/Shell打开后，由后端固定命令启动。使用独立exec，不写PTY、不重放输入、不请求提权、不source远端文件，远端返回值不得拼接为命令。共用5秒/32KiB限额，失败/取消不破坏主终端。SSH软件标识及系统类别仅作UI信息，不能参与密钥信任、ACL或算法降级。后端写前登记一次性标记，持久化失败时不发探测请求。设置只有布尔开关，拒绝任意脚本字段；主机归属仅由后端saved目标注入。SVG只能来自静态资源白名单，逐实例隔离内部ID；不接收远端SVG/路径/富文本。未执行真实SSH或安全运行验证。

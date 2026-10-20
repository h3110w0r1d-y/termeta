# 官方账号与浏览器登录

更新：2026-10-20。客户端、服务端和用户端已重构为浏览器登录与双模式入口；源码完成不代表真实 OAuth、MySQL 或三平台运行验收通过。

## 范围与模块

软件首次使用选择官方账号模式或本地/离线模式。官方入口提供 termeta.io、GitHub、Google，邮箱注册/登录只在独立用户端处理；桌面不再提供邮箱密码表单或身份绑定管理。未配置的第三方提供方不显示。用户端使用 Vue、Pinia、Vue Router、Naive UI，与管理端分离，浏览器不接收保险库密码或业务明文。

桌面账号 Token 只由 Rust 管理，创建保险库之前仅在进程内暂存，库可用后写入 SQLCipher 的独立账号设置；不返回 WebView、不写普通浏览器存储、不进入云快照。账号密码只用于网站认证，与保险库密码无关。桌面会话没有时间到期，`expires_at=null`；退出、管理员撤销、账号停用仍可使其失效。网页会话保持有限期限，Token 只保存在内存，刷新后重新登录。

官方模式的密码门禁、主密钥和自动同步见 [官方保险库同步](26-official-vault-sync.md)。本地模式完全不要求账号，第三方备份仅在本地模式设置中提供。退出官方账号保留本机保险库、数据和已运行会话。

## 登录协议

采用系统浏览器、S256 proof、一次性确认码和有限期轮询兑换。Rust 生成随机 verifier，只把 challenge 提交服务端；提供方凭据留在服务端。GitHub/Google 仍回调固定 HTTPS 服务，termeta.io 入口打开固定用户端 `/desktop/authorize?flow_id=...`，用户在网页注册/登录后显式确认授权当前桌面请求。

服务端完成身份认证后生成固定 `termeta://auth/callback?flow_id=<UUID>&provider=<termeta|github|google>#confirmation_code=<code>`。只携带短期确认码，不携带应用 Token、设备 proof 或提供方秘密；确认码在 URL 片段内。客户端必须匹配本进程已存在的 flow、提供方、发起窗口、代次与期限，再用设备 proof 和确认码共同兑换。新安装的 setup 与已存在的 locked 保险库也可从本地主窗口发起账号授权，登录不等于解锁。

| 入口 | 用途 |
| --- | --- |
| `/api/auth/providers` | 已启用的登录方式 |
| `/api/auth/oauth/{provider}/start` | 创建短期 PKCE 请求；桌面 audience 为 desktop |
| `/api/auth/oauth/termeta/authorize` | 网页已登录用户显式批准 termeta.io 桌面 flow |
| `/api/auth/oauth/{provider}/poll` | proof 与确认码约束的一次性兑换 |
| `/api/auth/oauth/{provider}/cancel` | proof 约束的取消 |
| `/api/auth/oauth/{provider}/browser-status` | 独立只读回执，不返回身份或 Token |
| `/api/auth/register`、`login`、`me`、`logout` | 独立网页账号管理；不再是桌面登录表单契约 |

GitHub/Google 用户端登录使用 audience=web 与固定 `/oauth/callback` 路由，网页自己生成 PKCE proof；提供方回调仅把确认码送回固定网页，Token 仍经 proof 兑换。回跳上下文不得从任意 return_to/URL 构造跳转。详细 HTTP DTO 以服务端认证文档为准。

管理端GitHub/Google登录使用独立audience=admin，固定回跳`TERMETA_OAUTH_ADMIN_BASE_URL/oauth/callback`；只接受已绑定且启用的管理员身份，不创建账号或要求纯第三方账号补设密码。管理会话有限期，每个管理API仍校验管理员权限。管理端仅临时保存自己的PKCE证明，不共享用户端Token或桌面永久会话；部署见管理端说明。

身份仍按提供方不可变 subject 匹配，已验证邮箱不能作为自动合并账号的理由；显式解绑必须至少保留一种可用登录方式。网页支持查看账号及解绑已有身份；新增绑定、找回账号密码、MFA 与忘记保险库密码清空云端尚未实现，不提供伪入口。

## 客户端 IPC 与权限

`src/services/desktop/` 是唯一 IPC 门面。保留账号摘要读取/刷新/退出、提供方查询及 start/confirm/poll/cancel；移除桌面 register/login/identities/unlink 命令，网页承担账号管理。

`account_owner` 允许本地 main 在 setup/locked/unlocked 管理登录，settings 仍必须已解锁。官方同步完成/初始化仅 main，设置窗口只展示同步状态和提示回主窗口完成门禁。账号登录代次与窗口代次独立于数据库初始化门闩；窗口关闭、取消和退出使旧 flow 无效，不把“库尚未解锁”当成取消正在进行的启动登录的理由。

固定服务来源默认 `https://termeta.io`，由编译期 `TERMETA_API_BASE_URL` 配置，组件不能传任意 URL。Rust 校验 HTTPS、固定授权域与路径、响应大小和期限，不自动重试账号操作或跟随重定向。跨窗口事件仅通知摘要失效，窗口重新读取，不广播秘密。

## 系统协议与生命周期

Tauri deep-link 插件只复用初始化和固定 scheme 注册，原始 URI 不广播至 WebView；macOS 原生 Opened、Windows/Linux single-instance 参数直接交给 Rust。协议 parser 拒绝未知/重复字段、非法 UUID、超长 URI 或确认码、不匹配的窗口/flow/提供方。冷启动没有设备 proof 时拒绝兑换，不自动新建 flow、打开终端、导航页面或解锁库。

回跳只向当前流程交接确认码并聚焦窗口，已有轮询负责兑换。自动唤起被浏览器或系统阻止时提供按钮与手动确认码备选。浏览器的失焦/隐藏不代表成功；只有服务端确认 flow 已消费才显示完成并尝试关闭。脚本关闭被拒绝时保留可手动关闭的提示。

独立回执只读、随机且受原流程期限限制，不可兑换账号；页面 no-store/no-referrer/CSP，禁止嵌入，不加载第三方资源。应用和反向代理日志不得记录确认码、proof、Token、提供方 code/state 或认证正文。既有回执迁移 003 保持原始字节，新模式与永久会话通过新增 004 迁移启用。

## 部署与验收

用户端与服务端以同源 `/api/` 代理部署，固定 HTTPS 公开来源与 OAuth 应用回调配置保持一致；SPA 路由不能吞掉 API callback。用户端部署示例见用户端 README，服务端运维见服务端认证部署。桌面运行前需由用户显式执行服务端迁移，不自动操作真实数据库。

仍需验证：真实提供方配置、邮箱冲突/身份隔离、proof/确认码错误、重复兑换/取消竞态、永久会话撤销、网页刷新/后台/BFCache、三平台协议安装与冷启动、浏览器阻止唤起/关闭、五语言及可访问性。静态检查与新增测试源码不能替代运行验收。

2026-10-19 决策：以 termeta.io 作为与 GitHub/Google 并列的浏览器登录方式，将账号表单与用户管理移至独立用户端；桌面账号永久可撤销，保险库密码门禁和官方自动同步独立编排，避免账号登录隐式暴露或覆盖本地数据。

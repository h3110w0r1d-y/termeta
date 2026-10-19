# 官方账号与第三方登录

更新：2026-10-19。本文为已落地源码的接口与安全契约，文档先于代码落地；源码完成状态在产品需求中维护，真实OAuth、MySQL与三平台联调尚未验收。

## 范围与模块

客户端在解锁本机保险库后，于设置的“账号”页注册、登录、查看当前账号、退出，并使用系统浏览器完成GitHub/Google登录。账号为可选能力，不改变本地终端无需账号的模式；不把账号密码当作保险库密码或备份密码。本轮不接入官方备份provider或更新插件。

服务端accounts保留邮箱密码认证；新增oauth领域，按models/schemas/providers/service/router分责，provider适配器隔离GitHub OAuth与Google OIDC，核心流程不依赖HTTP请求对象。服务端应用Client Secret仅从env加载，客户端不持有。管理端已有密码登录继续兼容，不自动赋予任何第三方账号管理员权限。

## 登录协议

采用“系统浏览器授权 + termeta:// 回跳 + 有限期轮询兑换”。Rust随机生成43–128字符verifier，以SHA256+Base64URL无填充得到S256 challenge，仅将challenge发给服务端。GitHub/Google仍回调固定官方HTTPS服务；服务端验证身份后，通过固定 `termeta://auth/callback?flow_id=<UUID>&provider=<github|google>#confirmation_code=<32字符Base64URL>` 自动唤起应用，并提供“返回 Termeta”按钮和手动确认码作为浏览器阻止唤起时的备选。URL只携带短期一次性确认码，不携带提供方code/state、设备verifier或会话Token；确认码位于片段，不发送到HTTP服务或Referer。服务器只保存其摘要。应用收到回跳后，由Rust匹配本机已存在的flow/provider及发起窗口、验证保险库仍解锁和代次/期限，再把确认码交给原有轮询兑换；设备proof与确认码缺一不可。

| 方法/路径 | 请求/响应 |
| --- | --- |
| GET `/api/auth/providers` | `{providers:[{id:"github"|"google",name:string}]}`，仅返回已配置提供方 |
| POST `/api/auth/oauth/{provider}/start` | `{code_challenge,code_challenge_method:"S256"}` → `{flow_id,authorize_url,expires_at,poll_interval:3}` |
| POST `/api/auth/oauth/{provider}/poll` | `{flow_id,code_verifier,confirmation_code?}`，完成授权后需确认码 → `{status:"pending"|"completed"|"denied"|"expired",token:TokenView|null,error:string|null}` |
| POST `/api/auth/oauth/{provider}/cancel` | `{flow_id,code_verifier}`，不需要确认码，验证proof后取消，204 |
| GET `/api/auth/oauth/{provider}/callback` | 提供方code/state回调，成功返回固定协议回跳页及确认码备选，失败返回静态网页，不向浏览器发Termeta Token |
| POST `/api/auth/oauth/{provider}/browser-status` | 回调页专用 `{flow_id,receipt}` → `{status:"pending"|"completed"|"unavailable"}`，只读且不返回身份/Token，不改变客户端 poll |
| POST `/api/auth/oauth/{provider}/link/start` | 需Bearer，challenge请求与响应同start，发起显式身份绑定 |
| GET `/api/auth/identities` | 当前账号`{identities:[{provider,created_at}]}` |
| DELETE `/api/auth/identities/{provider}` | 显式解绑，保留至少一种可用登录方式，204 |

普通注册/登录仍使用`POST /api/auth/register|login`，JSON`{email,password}`，返回现有TokenView；`GET /api/auth/me`及logout维持原接口。UserView新增`has_password:bool`，旧字段不删除。绑定flow的poll成功返回completed和token=null；客户端重新读取账号摘要及identities，不能把绑定误当作切换账号。

服务端flow只保存state摘要、challenge、provider交换所需的独立PKCE verifier/nonce、到期时间、用途及账号关联。state一次性claim后才联网，终态不可重放；最多10分钟，轮询按行锁执行3秒间隔限制。OAuth provider Token只在本次回调的内存使用，不持久化；确认码匹配后才原子创建Termeta会话并消费flow；缺少确认码继续pending，错误码使flow终止且需重新授权，响应丢失不能重放，重新发起登录即可。取消/超时与回调并发时不能复活flow。

GitHub通过固定user和user/emails接口取得不可变user id及已验证邮箱；Google验证ID Token签名、固定issuer、audience、有效期、nonce和email_verified，以sub标识身份。身份唯一键(provider,subject)，新账号要求已验证邮箱。已有邮箱绝不自动合并，提示先登录既有账号再显式绑定；同一身份不能绑定不同账号。解绑最后一种登录方式必须拒绝。提供方未配置时不显示对应按钮，默认关闭。

## 客户端IPC契约

唯一门面仍为src/services/desktop。Rust负责HTTPS、凭据、系统浏览器与PKCE；Vue仅持有账号摘要、忙碌与错误状态。命令仅允许本地main/settings且保险库已解锁，经AppManifest、handler与capability同时注册。

| 命令 | 参数 / 结果 |
| --- | --- |
| `get_account_status` | 无 → `{user:AccountUser|null,expiresAt:string|null}`；只读本机缓存，不主动联网；有效性由refresh验证 |
| `refresh_account` | 无 → 同status，调用me验证服务端会话 |
| `list_account_providers` | 无 → `AccountProvider[]`，包含id/name |
| `register_account` / `login_account` | `email,password` → AccountStatus |
| `logout_account` | 无 → 无；即使网络失败也先清除本机会话，并报告远端撤销失败 |
| `start_account_oauth` | `provider:"github"|"google",link?:bool` → `{flowId,provider,expiresAt,pollInterval}`，Rust打开受校验浏览器URL |
| `confirm_account_oauth` | `flowId,confirmationCode` → 无；仅写入当前窗口flow内存，不立即联网，后续poll携带 |
| `poll_account_oauth` | `flowId` → `{status:"pending"|"completed"|"denied"|"expired",account:AccountStatus|null,error:string|null}` |
| `cancel_account_oauth` | `flowId` → 无，立即失效本地proof并尽力取消远端 |
| `list_account_identities` | 无 → `{provider,createdAt}[]` |
| `unlink_account_identity` | `provider` → 无 |

AccountUser使用camelCase：`{id,email,isAdmin,hasPassword,createdAt}`。Rust settings.account版本1保存绑定固定官方服务来源的Token、expiresAt与用户摘要；Token留在本机SQLCipher，不返给WebView、不进入任何备份。服务地址由编译期`TERMETA_API_BASE_URL`配置，默认`https://termeta.io`，仅HTTPS、无凭据/查询/片段，不能由组件指定任意目标。

Rust网络不自动重试或重定向，证书验证、超时与响应字节预算；错误脱敏，不透出请求体/认证URL。短期操作门闩和代次隔离并发登录/退出，普通全局锁不跨网络等待。flow绑定发起窗口，一次只保留有限待定项；窗口关闭/退出/保险库锁定使旧结果无效。跨窗口account-changed只传失效通知，各窗口重新读取摘要。

## 界面与验收

Naive UI、AppDialog footer、统一CommandRegistry及五语言语义key；表单model/v-model/rules共享字段，浏览器授权成功后自动回到发起窗口并完成兑换；手动确认码默认收起，仅用于自动回跳受阻时，提交后清除输入。取消浏览器登录不会自动重放。用户可刷新账号状态、取消等待、显式绑定/解绑并查看登录方式；没有验证码邮件、找回密码或MFA能力，不用无效按钮伪装已完成。

上线前由运维配置GitHub/Google应用、固定HTTPS回调、部署公开来源并关闭含code/state的访问日志。需要验证state/nonce/PKCE/确认码错误、单次兑换/并发取消、邮箱冲突/绑定隔离、最后登录方式保护、Token过期/注销、DB迁移恢复、网络超时、窗口关闭及五语言三平台浏览器行为。本轮不运行构建/项目测试或真实外部连接。

服务端通过独立 `termeta.oauth` 脱敏事件记录授权生命周期、flow UUID、失败阶段/原因、HTTP 状态及白名单提供方错误码，不输出 URL、认证秘密、身份信息或原始异常。浏览器保留统一失败文案；排查使用服务端诊断文档，不能靠重新开启原始访问日志定位问题。日志与脱敏用例源码已补齐，真实故障和代理日志边界仍待联调。

## 部署接入

部署方在服务端配置`TERMETA_OAUTH_PUBLIC_BASE_URL`和对应`TERMETA_OAUTH_GITHUB_CLIENT_ID/CLIENT_SECRET`、`TERMETA_OAUTH_GOOGLE_CLIENT_ID/CLIENT_SECRET`，回调分别为固定来源下的`/api/auth/oauth/github/callback`、`/api/auth/oauth/google/callback`；提供方默认关闭，无预置凭据。新增`002_oauth_identities.sql`和`003_oauth_browser_receipts.sql`通过既有显式运维迁移执行，不能改写历史迁移。完整环境与控制台设置见服务端认证文档。

桌面默认访问`https://termeta.io`，使用其他部署时须在编译环境设置`TERMETA_API_BASE_URL`，并与公开来源和提供方回调保持一致；该值不是客户端Secret。账号入口位于主窗口保险箱导航底部与设置“账号”页。登录有效性通过“刷新账号状态”验证，缓存不代表服务端会话仍有效；切换账号先退出。更改服务来源后旧Token不会发给新来源，可使用退出入口清除旧记录再登录。

2026-10-19 安全取舍：以固定系统协议自动传回独立浏览器确认码，替代默认手工复制，保留设备proof与一次性确认码双重校验。转发授权URL后，浏览器仅唤起授权者本机应用；没有对应在途flow的应用拒绝处理，不为外部链接创建flow、切换账号、解锁保险库或发起网络请求。确认码无恢复/重放接口，丢失时取消重发。

## 系统协议与生命周期

Tauri官方deep-link插件声明desktop scheme `termeta`，仅复用其初始化/固定scheme注册能力；不接入其会向WebView广播原始URI的事件和JS API钩子。Windows/Linux由single-instance原生参数转交现有进程，macOS由原生Opened事件直接进入Rust。single-instance不启用deep-link feature，避免重复处理与原始URI广播。安装注册由打包配置完成；Linux运行时也注册以覆盖AppImage，Windows仅开发模式额外注册当前程序，注册失败保留手动确认码入口。macOS通过已安装app bundle的URL scheme注册，不能把裸开发进程视为已注册应用。WebView不获得deep-link读取或注册权限。

Rust仅接受固定scheme/host/path、无用户信息/端口、恰好一个flow_id/provider及片段confirmation_code；拒绝重复字段、未知字段、非法UUID、超长URI和无效确认码。服务端RFC3339到期时间转换为Rust单调时钟截止点，上限十分钟；仅匹配本进程内尚未过期的授权，并聚焦其owner窗口；重复同码幂等，冲突码拒绝覆盖。冷启动没有设备proof，拒绝兑换，用户需解锁后重新发起；取消、窗口关闭、退出或保险库锁定后也拒绝迟到回跳。外部URI不得变成通用命令、任意页面导航、文件路径或终端输入。

服务端仅构造固定回跳目标，不接受return_to或客户端任意scheme；成功页以简洁居中的独立 HTML/CSS/JS 呈现等待/完成/失效状态，使用随机 CSP nonce 许可的脚本唤起固定 URI 并查询同源只读回执，不加载第三方资源，保持 no-store/no-referrer 与禁止嵌入。应用和代理日志不能记录回跳URI/片段；浏览器/操作系统可能提示允许打开应用，自动唤起被拦截时使用页面按钮或展开手动确认码。上线前验证三平台安装注册、已运行/第二实例/冷启动、重复/伪造回跳、锁定/取消/过期和浏览器阻止唤起等行为。

2026-10-19 原生接收取舍：官方插件默认通过全局event转发完整URI，即使不授予其get_current权限，已有core:event监听权限的WebView仍可能接收。采用只代理Plugin.initialize的薄适配，协议注册仍使用官方实现；运行中回跳通过Rust原生事件/单实例入口消费，不委托默认广播。升级插件时必须复核初始化是否引入新的运行期事件或JS钩子。


## 回跳页回执与自动关闭

2026-10-19 产品与安全取舍：浏览器没有可靠的自定义协议打开成功事件，页面失焦/隐藏不能证明应用已打开。callback 验证成功后生成另一份独立随机只读回执，数据库仅存摘要，页面通过同源 `browser-status` 查询本次 flow。只有服务端原子完成兑换、flow 成为 consumed 时才返回 completed；它证明服务端已收到正确设备 proof 和浏览器确认码，不证明客户端响应接收/Token 持久化或操作系统唤起成功。手动确认码完成授权同样适用，无需新增客户端 IPC 或回执上报。

页面确认 completed 后显示完成状态并尝试一次自动关闭，后台标签的轮询/关闭可能受浏览器节流；浏览器拒绝关闭时保留“可关闭标签页”和关闭按钮。系统浏览器打开的普通标签通常不具备脚本关闭权限，不通过重开窗口或篡改历史绕过限制。取消/过期显示重发说明，网络异常限时重试，均不自动当作完成；禁用 JS 时仍保留固定回跳链接与手动确认码，但无法自动查询或关闭。

回执不能兑换账号、读取邮箱或会话，只存在本次回调页和查询正文，不进入地址、Cookie、本地持久化或日志。读回执和设备 poll 分别节流，不抢占客户端兑换；回执受同一 flow 的原有效期约束，终态不续期，页面到期停止查询并隐藏确认码/回跳操作。服务端 003 迁移必须在新代码上线前应用，代理同源转发此 POST 且禁止记录认证正文。运行验收增加：回执完成/取消/到期并发、429/超时、后台/BFCache、关闭被浏览器拒绝、禁用 JS/剪贴板、窄屏/深浅色与键盘可访问性。

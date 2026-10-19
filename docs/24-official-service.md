# Termeta 官方服务与接入边界

更新：2026-10-19。官方服务端是独立项目，入口为 README 和 协作指南。不替代桌面 Rust IPC、会话 actor 或本机保险库，不要求本地功能登录账号。

## 当前范围

Python 3.14、FastAPI、Tortoise ORM、外部 MySQL，Docker Compose 只部署API。已有账号注册/登录/退出、可撤销Bearer会话、账号隔离加密备份版本与配额、Tauri updater动态响应、公开软件分发和 `/api/admin/releases` 管理接口基础代码。独立Vue + Naive UI管理端已有管理员登录、版本列表/详情及草稿/发布/撤回/删除界面和真实接口接入源码，尚未启动或联调；账号管理、全站备份管理和远程协助管理尚未实现。服务端按accounts/oauth/backups/releases分领域，并分离models/schemas/service/router；ORM生命周期遵循Tortoise官方FastAPI示例，详见服务端架构。依赖默认实施时最新稳定版，使用pip与requirements清单，Docker先安装依赖再复制代码以复用缓存，最新版不等于无漏洞。

目前未运行服务、迁移、测试或Docker构建，未做真实MySQL、客户端联调或发布验收。客户端尚未安装/接入 `tauri-plugin-updater`、`@tauri-apps/plugin-updater`，官方备份provider尚未接入；官方账号的邮箱注册/登录与GitHub/Google登录已有客户端和服务端源码，运行验收仍待完成，见[账号认证](25-account-authentication.md)。

## 官方备份与同步

官方 `termeta.io` 备份和Gist/WebDAV为独立目标，账号登录Token与第三方凭据分开。复用现有[备份加密格式](22-data-backup.md#3-包格式与密码)，客户端继续白名单SQLite快照 → Zstd → Argon2id/AES-256-GCM → Base64 `TMETABK1`。备份密码、主密钥及明文快照不上传，服务器只能做封套结构校验，不能代替密码认证。

服务器保留不可变全量快照版本，多设备通过列表和下载交换快照。恢复继续解密预览、用户确认和仅新增合并，不静默覆盖。字段级双向同步、自动备份、冲突合并和客户端首次官方云初始化尚未实现。API契约以服务端备份文档为准。

## 分发与更新

服务端 `/api/updates/{target}/{arch}/{current_version}` 遵循Tauri官方动态协议：无新版本204，有更新200 JSON携带version/url/signature及可选notes/pub_date；签名为.sig文件内容，制品是HTTPS地址。签名私钥仅在发布环境，不上传API；客户端将来必须内置公钥校验，不能绕过签名。

管理员创建草稿后显式发布，可撤回；当前仅稳定渠道。实际制品上传、签名/公证、CDN与正式发布流水线由后续工作落实，详见服务端分发文档。

## 远程协助（仅规划）

目标为用户显式将一个终端经服务器临时共享，受邀者用一次性SSH授权连接，发起者可撤销。规划使用独立中继与出站隧道，不暴露客户端任意端口、不上传原主机凭据；默认只读，控制另行授权，禁止自动广播远程输入。TTL、一次认领、撤销、主机身份核验、背压、数据与安全事件边界详见服务端设计。本轮没有实现SSH gateway、会话路由或客户端共享按钮。

2026-10-19 决策：保持桌面本地优先，新增可选独立官方服务作为账号、官方密文备份及更新控制面；未来远程协助不进入当前本地终端核心默认路径。

2026-10-19 官方账号扩展采用系统浏览器、termeta://自动回跳及S256轮询兑换；固定HTTPS回调验证身份后将一次性确认码传回本机在途授权，未匹配时拒绝。Client Secret和提供方Token留在服务端，应用Token留在客户端Rust/SQLCipher；相同邮箱不自动合并，身份绑定要求既有会话。部署必须配置真实应用与固定HTTPS回调并应用新增迁移，详见服务端认证部署。

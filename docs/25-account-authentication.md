# 官方账号与浏览器登录

更新：2026-10-20。本文维护桌面登录、Token、回跳和授权边界；真实 OAuth/MySQL、浏览器与三平台运行验收尚未完成。

## 范围与模块

桌面提供 termeta.io、GitHub、Google 浏览器入口，未配置的提供方不显示。邮箱注册/登录和基础账号管理在 独立用户端，桌面不提供账号密码表单。网页不接收保险库密码或业务明文。

桌面 Token 只由 Rust 管理：保险库创建前暂存在进程内，可用后写入 SQLCipher 独立账号设置；不返回 WebView、不写浏览器存储、不进入云包。桌面会话不按时间到期，退出、管理员撤销或账号停用仍使其失效。网页/管理会话有限期，与桌面会话隔离。

账号认证不等于解锁保险库。官方密码门禁和自动同步见 [官方同步](26-official-vault-sync.md)；本地模式无需账号，退出保留本地保险库、数据和运行会话。

## 登录协议

使用系统浏览器、S256 proof、短期确认码与一次性兑换。Rust 生成随机 verifier，仅向服务端提交 challenge；提供方凭据留在服务端。termeta.io 打开固定用户端 `/desktop/authorize`，用户注册/登录后显式授权；GitHub/Google 回调固定 HTTPS 服务。

认证后服务端生成固定 `termeta://auth/callback?flow_id=<UUID>&provider=<termeta|github|google>#confirmation_code=<code>`。URI 只携带短期确认码，不含 Token、设备 proof 或提供方秘密。Rust 必须匹配本进程在途 flow、提供方、发起窗口、代次和期限，再用 proof 与确认码兑换；取消或关闭使旧流程失效。

身份按提供方不可变 subject 匹配，不能按相同邮箱自动合并。管理端使用独立 `admin` audience 和固定来源，仅接受已有管理员身份，后端逐请求校验权限。身份管理及 HTTP DTO 以 服务端认证契约 为准；新增绑定、账号密码找回、MFA 和忘记保险库密码清空云端尚未实现。

## 客户端 IPC 与权限

账号摘要/刷新/退出、提供方查询及授权 start/confirm/poll/cancel 统一经过 `src/services/desktop/`。`account_owner` 允许本地 main 在 setup/locked/unlocked 发起授权，settings 必须已解锁；账号登录不扩大业务数据或会话权限，官方门禁完成仅 main。

固定服务来源默认 `https://termeta.io`，仅由编译期 `TERMETA_API_BASE_URL` 配置，组件不能提交任意 URL。Rust 校验 HTTPS、授权域/路径、响应大小及期限，禁自动重试与重定向；跨窗口只通知摘要失效，不广播秘密。库尚未解锁不能误取消当前启动登录，迟到结果不能恢复旧账号流程。

## 系统协议与生命周期

Tauri deep-link 仅复用固定 scheme 注册，原始 URI 不广播 WebView。macOS Opened 和 Windows/Linux single-instance 参数直接交给 Rust。parser 拒绝未知/重复字段、非法 UUID、超长 URI/确认码以及不匹配流程；冷启动没有设备 proof 时拒绝兑换，不自动新建授权、导航、打开终端或解锁库。

回跳交接确认码并聚焦窗口，由既有轮询兑换；系统阻止唤起时保留按钮和手动确认码。浏览器失焦/隐藏不代表成功，只有服务端确认 flow 已消费才显示完成并尝试关闭，关闭失败则提示手动关闭。

浏览器回执只读、随机、短期且不可兑换账号；页面使用 no-store/no-referrer/CSP，不允许嵌入或第三方资源。应用及代理日志不得记录确认码、proof、Token、提供方 code/state 或认证正文。

## 部署与验收

用户端与服务端同源部署，公开 HTTPS/OAuth 回调须一致，SPA 路由不能吞掉 API。部署和新增迁移遵循 服务端认证文档，已应用迁移保持原始字节，由用户显式执行真实数据库迁移。

运行验收覆盖真实提供方与身份隔离、proof/确认码错误、重复兑换/取消竞态、会话撤销、浏览器生命周期和三平台协议安装/唤起，统一见 [验收边界](01-product-requirements.md#11-运行验收边界)。

2026-10-19 决策：账号表单与用户管理移至独立用户端，桌面统一浏览器登录及永久可撤销会话；保险库门禁独立，避免账号登录隐式暴露或覆盖本地数据。

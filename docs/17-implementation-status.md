# 首期实施状态与验证记录

日期：2026-10-15。范围：M1 SSH 纵向切片与 M2 SSH MVP 的源码实施。**当前为实现待验证，不能宣布 MVP 验收通过。** 代理没有构建、启动应用、连接真实主机或运行 Rust 测试。用户已提供 macOS 启动失败日志，见本文后续修复记录。

## 1. 实际交付

| 模块 | 源码入口 | 已写入的行为 |
| --- | --- | --- |
| 领域核心 | [core](../crates/terminal-core/src/lib.rs) | 领域 DTO、工厂端口、每会话任务、owner、request 幂等、20 活动/4 建立并发、取消、输入去重、challenge 与终态摘要 |
| SSH | [SSH](../crates/terminal-ssh/src/lib.rs) | DNS/TCP/KEX、SHA256 指纹、未知密钥确认/变更阻断、密码/OpenSSH 私钥/交互认证、PTY/Shell 成功回复、resize、KeepAlive、取消关闭 socket |
| 流控 | [二进制流](../crates/terminal-core/src/stream.rs) | 24 字节头、连续序号/偏移、32 KiB 帧、256 KiB 信用、512 KiB 输出字节配额、64 KiB 输入、累计消费 ACK、结束前冻结 finalOffset |
| 存储 | [SQLite](../crates/terminal-storage/src/lib.rs) | 版本迁移、乐观锁主机/分组、标签/收藏/搜索、持久信任、设置、历史、私钥引用与凭据元数据 |
| 凭据 | [安全适配](../crates/terminal-security/src/lib.rs) | 平台 keyring、显式保存/删除/恢复、pending/ready/deleting 协调；不返回秘密、不做明文降级；私钥限大小/权限/内容授权摘要复核 |
| 桌面壳 | [原生命令](../src-tauri/src/commands/mod.rs) | main 窗口身份、资源 owner 校验、参数验证、原生选择私钥、外链白名单、Channel 桥、历史收尾、失联取消、单实例 |
| 前端工作区 | [应用](../src/App.vue) | 主机管理、分组/收藏/标签/搜索、快速连接、认证与信任弹窗、多标签、关闭确认、错误状态、历史管理 |
| 终端宿主 | [Registry](../src/modules/terminal/registry.ts) | 稳定 terminalId、隐藏保活、失败保留、原终端重连、解析后 ACK、FIFO 输入、UTF-8 二进制、搜索/复制/多行粘贴确认、全屏最后画面有界文本快照 |
| 设置 | [设置](../src/modules/settings/SettingsDialog.vue) | 应用外观、字体/滚屏、8 内置终端方案、颜色编辑/JSON、profile 和当前终端覆盖、快捷键分类/录制/多绑定/导入导出、信任/凭据管理 |

所有终端正文留在 xterm/有界字节管线，未进入 Pinia、SQLite、日志或浏览器持久存储。Local、Serial、SFTP、隧道、Agent、跳板和分屏未实现。

## 2. 本轮契约与方案调整

- 根 Cargo workspace 管理四个领域 crate 与 Tauri 壳，release profile 与 Cargo.lock 移到根目录。
- 首期只注入 SSH 工厂，保留 SessionFactory 端口；尚未为未实现的协议建立可用工厂注册表或空 crate。
- `open_session` 增加稳定视图 `terminalId`；后端 sessionId 每次重连重新分配。相同 owner/requestId 返回原快照，不替换原 Channel。
- `save_host` / `save_host_group` 使用独立 `expectedRevision`，壳将其写入领域 draft；删除携带 revision。
- 自定义配色、快捷键和 `hostSchemes` 以单个版本化 `app` 设置 JSON 保存，未建独立方案/主机偏好表；Rust 做字段/长度/颜色/引用/动作白名单校验。设置没有跨窗口 revision 契约，首期只允许主窗口。
- 认证 challenge 仅本次使用，`savePolicy` 只接受 `never`；用户在主机编辑中显式调用 `save_credential`，得到引用再绑定认证计划；OTP 不允许保存。无通用 `get_secret` 命令。
- 私钥原生选择返回 `{keyFileRef,name}`，不返回路径、摘要或正文。文件变化需重新选择授权。仅支持 OpenSSH 私钥，不宣称支持 PPK/证书/Agent。
- 配色/快捷键导出通过 `export_json({name,value})` 原生保存对话框写入所选路径，前端不能提供路径；补充 `list_history` / `clear_history` / `recover_credential`；新增命令均同步 manifest、handler 与 capability。
- CSP 保留 `style-src 'unsafe-inline'` 供 xterm/Reka 动态布局，脚本不允许 eval/远端资源；devCsp 单独列出精确开发 origin。发布 CSP 不带 HMR。

变更原因见 [ADR-017/018](12-decisions.md#adr-017首期实际依赖与凭据适配)。

## 3. 静态验证证据

本轮仅执行不构建/运行项目的检查：

- `pnpm install --ignore-scripts`：安装前端依赖并更新锁文件，禁用生命周期脚本。
- `pnpm exec vue-tsc --noEmit`：前端严格类型静态检查；不调用 Vite/build。
- `pnpm exec prettier --write/--check`、`cargo fmt --all` 与 `cargo fmt --all -- --check`：格式化/语法解析。
- `cargo generate-lockfile`、`cargo fetch --locked`、`cargo metadata --no-deps --locked --format-version 1`：依赖解析、下载与清单读取，不编译。
- Rust 固定 release 源码核对：russh Handler/AuthResult/PTY/算法、Tauri raw Channel/宏/ACL、SQLx 事务/迁移、keyring 平台 features。
- 自定义命令 manifest/handler/capability/前端调用集合、JSON/TOML、Markdown 本地链接与围栏、`git diff --check` 静态检查。
- 已编写 [core fake driver 测试](../crates/terminal-core/src/tests.rs)，覆盖 owner/幂等、字节帧/ACK/输入去重与 challenge 单次消费；**未执行**。

上述类型、格式、锁清单、JSON/TOML、链接/围栏、差异与31个ACL命令集合静态检查已通过；详细记录见路线。任何“静态通过”均不证明 Rust 类型检查、Tauri 宏展开、SSH 互通、WebView 行为或性能正确。

## 4. 依赖与公告核查

主依赖：russh 0.64.1（ring+rsa，无默认 features，Apache-2.0，声明 MSRV 1.89）、SQLx 0.8.6、keyring 3.6.3（按平台显式后端）、Tauri 2.11.5/build 2.6.3；前端 xterm 6.0.0、Fit 0.11.0、Search 0.16.0、WebLinks 0.12.0、Reka 2.11.0、Pinia 4.1.0。精确传递版本以两个锁文件为准，不混用 latest API。

2026-10-15 的 `pnpm audit --json` 返回 0 个公告；不构成供应链安全证明。OSV querybatch 对 Rust 锁文件中 688 个注册依赖查询，发现以下条目，**发布前安全门槛尚未通过**：

| 依赖 | 公告 | 处理状态 |
| --- | --- | --- |
| rsa 0.9.10 / 0.10.0-rc.18 | [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html) | RSA 时序侧信道，无已发布修复；实际 SSH 使用 rc.18 的 PKCS1v1.5 SHA256/512 签名→RSA 私钥运算，签名接口未开启 blinding；不直接暴露 PKCS1 解密 oracle，但不能据此认为侧信道安全。0.9.10 来自 lock 中可选 SQLx/MySQL 分支，cargo tree 的 Linux 启用树未包含它 |
| glib 0.18.5 | [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) / GHSA-wrw7-89jp-8q8g | Linux GTK3 传递依赖 VariantStrIter 不安全；修复版本跨不兼容 minor，需桌面栈升级或经审阅补丁与 Linux 验证 |
| proc-macro-error 1.0.4 | [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) | 上游停止维护，跟踪桌面依赖升级 |
| unic-char-property / unic-char-range / unic-common / unic-ucd-ident / unic-ucd-version | RUSTSEC-2025-0081/0075/0080/0100/0098 | 上游停止维护，跟踪桌面依赖替代；与可利用漏洞公告分开记录 |

不通过忽略公告、伪造“已修复”或编译成功来宣称安全。未生成完整传递依赖 notices；真实构建工具链/MSRV 与三平台原生依赖仍待授权核验。

## 5. 仍需授权运行验收

[验证计划](11-validation.md) 的 SP-01~05、SP-07~09 均待执行。尤其需要验证：错误/变更密钥在发送秘密前拒绝、多因素认证、洪泛/无 ACK 下取消和 KeepAlive、公钥算法兼容、重连旧字节/解析模式隔离、全屏快照、IME/焦点/链接鼠标报告去重、剪贴板、CSP、窗口关闭、OS keyring 与重启持久化、迁移/备份恢复。

当前全屏快照为有界可读文本，非保留鼠标/光标/样式的完整可交互屏幕。WebView/操作系统未实际验收；Linux 系统凭据服务和 Windows 文件 ACL/路径替换边界仍需平台复核。现有历史查询最多 1000 条、按 90 天清理；终端正文不落盘。配置保存没有无条件“安全擦除”承诺。

M1/M2 只能在相应 P0 与运行矩阵有实际证据后改为“已验证”。本记录不授权启动、连接、编译、打包或发布。

## 6. 用户启动反馈与修复（2026-10-15）

用户 macOS pnpm tauri dev 日志显示：Tauri JS/Rust minor 版本不匹配，随后可执行程序进入 setup，数据库迁移失败。只读对照已应用 v1 SHA384 与源码，确认同编号 migration 曾被增加私钥摘要列。现已恢复 v1 原文，新增 v2 列迁移、升级备份与旧私钥引用拒绝；迁移错误保留具体原因。未修改实际数据库，下一次启动由应用正常迁移。

前端 @tauri-apps/api 固定 2.11.0，原生 Tauri 仍为 2.11.5；未使用的 @tauri-apps/plugin-opener 从 JS 依赖移除，原生受控 opener 保留。新增2项存储回归测试源码，尚未执行。代理仅执行依赖安装（ignore-scripts）、格式、类型与元数据检查，修复后启动仍待用户复验。

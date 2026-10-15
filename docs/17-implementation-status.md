# 首期实施状态与验证记录

日期：2026-10-15。范围：M1 SSH 纵向切片与 M2 SSH MVP 的源码实施。**当前为实现待验证，不能宣布 MVP 验收通过。** 代理没有构建、启动应用、连接真实主机或运行 Rust 测试。用户已进入终端并反馈体验问题。此前启动失败和旧库迁移记录为历史；当前凭据/数据库契约以第10节及 ADR-022 为准。

## 1. 实际交付

| 模块 | 源码入口 | 已写入的行为 |
| --- | --- | --- |
| 领域核心 | [core](../crates/terminal-core/src/lib.rs) | 领域 DTO、工厂端口、每会话任务、owner、request 幂等、20 活动/4 建立并发、取消、输入去重、challenge 与终态摘要 |
| SSH | [SSH](../crates/terminal-ssh/src/lib.rs) | DNS/TCP/KEX、SHA256 指纹、未知密钥确认/变更阻断、密码/OpenSSH 私钥/交互认证、PTY/Shell 成功回复、resize、KeepAlive、取消关闭 socket |
| 流控 | [二进制流](../crates/terminal-core/src/stream.rs) | 24 字节头、连续序号/偏移、32 KiB 帧、256 KiB 信用、512 KiB 输出字节配额、64 KiB 输入、累计消费 ACK、结束前冻结 finalOffset |
| 存储 | [SQLite](../crates/terminal-storage/src/lib.rs) | 全新 schema 1、乐观锁主机/分组、标签/收藏/搜索、持久信任、设置、历史、公共凭据与主机专属密码元数据 |
| 凭据 | [安全适配](../crates/terminal-security/src/lib.rs) | 平台 keyring、显式保存/删除/恢复、pending/ready/deleting 协调；不返回秘密、不做明文降级；私钥限大小/权限/内容授权摘要复核 |
| 桌面壳 | [原生命令](../src-tauri/src/commands/mod.rs) | main 窗口身份、资源 owner 校验、参数验证、原生选择私钥、外链白名单、Channel 桥、历史收尾、失联取消、单实例 |
| 前端工作区 | [应用](../src/App.vue) | 主机管理、分组/收藏/标签/搜索、快速连接、认证与信任弹窗、多标签、关闭确认、错误状态、历史管理 |
| 终端宿主 | [Registry](../src/modules/terminal/registry.ts) | 稳定 terminalId、隐藏保活、失败保留、原终端重连、解析后 ACK、FIFO 输入、UTF-8 二进制、搜索/复制/多行粘贴确认、全屏最后画面有界文本快照 |
| 设置 | [设置](../src/modules/settings/SettingsDialog.vue) | 应用外观、字体/滚屏、64套提取内置终端方案、颜色编辑/JSON、profile 和当前终端覆盖、快捷键分类/录制/多绑定/导入导出、信任/凭据管理 |

所有终端正文留在 xterm/有界字节管线，未进入 Pinia、SQLite、日志或浏览器持久存储。Local、Serial、SFTP、隧道、Agent、跳板和分屏未实现。

## 2. 本轮契约与方案调整

- 根 Cargo workspace 管理四个领域 crate 与 Tauri 壳，release profile 与 Cargo.lock 移到根目录。
- 首期只注入 SSH 工厂，保留 SessionFactory 端口；尚未为未实现的协议建立可用工厂注册表或空 crate。
- `open_session` 增加稳定视图 `terminalId`；后端 sessionId 每次重连重新分配。相同 owner/requestId 返回原快照，不替换原 Channel。
- `save_host` / `save_host_group` 使用独立 `expectedRevision`，壳将其写入领域 draft；删除携带 revision。
- 自定义配色、快捷键和 `hostSchemes` 以单个版本化 `app` 设置 JSON 保存，未建独立方案/主机偏好表；Rust 做字段/长度/颜色/引用/动作白名单校验。设置没有跨窗口 revision 契约，首期只允许主窗口。
- 认证 challenge 仅本次使用，`savePolicy` 只接受 `never`；公共凭据由凭据页调用 `save_credential` 创建，主机专属密码通过 `save_host` 原子绑定；OTP 不允许保存。无通用 `get_secret` 命令。
- 私钥原生选择返回 `{keyFileRef,name}`，不返回路径、摘要或正文。文件变化需重新选择授权。仅支持 OpenSSH 私钥，不宣称支持 PPK/证书/Agent。
- 配色/快捷键导出通过 `export_json({name,value})` 原生保存对话框写入所选路径，前端不能提供路径；补充 `list_history` / `clear_history` / `recover_credential`；新增命令均同步 manifest、handler 与 capability。
- CSP 保留 `style-src 'unsafe-inline'` 供 xterm/Naive UI 动态布局，脚本不允许 eval/远端资源；devCsp 单独列出精确开发 origin。发布 CSP 不带 HMR。

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

主依赖：russh 0.64.1（ring+rsa，无默认 features，Apache-2.0，声明 MSRV 1.89）、SQLx 0.8.6、keyring 3.6.3（按平台显式后端）、Tauri 2.11.5/build 2.6.3；前端 xterm 6.0.0、Fit 0.11.0、Search 0.16.0、WebLinks 0.12.0、Naive UI 2.45.3、Pinia 4.1.0。精确传递版本以两个锁文件为准，不混用 latest API。

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

用户 macOS pnpm tauri dev 日志显示：Tauri JS/Rust minor 版本不匹配，随后可执行程序进入 setup，数据库迁移失败。只读对照已应用 v1 SHA384 与源码，确认同编号 migration 曾被增加私钥摘要列。现已恢复 v1 原文，新增 v2 列迁移、升级备份与旧私钥引用拒绝；迁移错误保留具体原因。未修改实际数据库。此迁移方案现已被第10节全新 schema 基线取代，不再提供旧库升级。

前端 @tauri-apps/api 固定 2.11.0，原生 Tauri 仍为 2.11.5；未使用的 @tauri-apps/plugin-opener 从 JS 依赖移除，原生受控 opener 保留。新增2项存储回归测试源码，尚未执行。代理仅执行依赖安装（ignore-scripts）、格式、类型与元数据检查，修复后启动仍待用户复验。

## 7. 首期工作区重构（2026-10-15）

用户确认本轮保持 SSH 首期范围，SFTP 交付固定双栏布局，文件服务和传输仍为 M3。App 改为首页、SFTP、SSH 的统一标签栏，首页/SFTP 不可关闭；标签栏可在设置切换顶部或左侧并持久化。首页内部菜单为主机、凭据、设置，主机列表使用宽内容区，凭据独立管理，设置嵌入首页。所有终端宿主独立保活，固定页不创建终端实例；原生下拉框显式尺寸修复待 WebView 验证。

SSH 按服务器支持的方法与 partial success 自动衔接 Keyboard-interactive；删除主机编辑的追加 OTP 字段，兼容旧 authPlan。认证提交只消费对应 challengeId，多轮提示更新输入焦点；延迟焦点任务检查当前活动终端及模态状态，标签方向键导航保留标签焦点。认证期间布局变化在 PTY 建立后补发尺寸。设置支持放弃修改、保存快照及延迟加载合并，防止覆盖已加载偏好。新增 app.tabPosition 类型与 Rust 白名单，旧设置缺失默认 top；未新增依赖、migration 或 IPC 命令。

本轮运行验收尚未执行，不宣称首期通过验收。重点待验证项见 [验证计划](11-validation.md) 的重构补充。

本轮最终静态检查：`pnpm exec vue-tsc --noEmit`、`pnpm exec prettier --check src`、`cargo fmt --all -- --check`、`git diff --check` 通过；19 份 Markdown 的代码围栏与 142 个本地文件链接目标检查通过（未校验锚点）。未构建、启动、运行测试或连接 SSH，未进行 WebView 视觉验收。

## 8. Naive UI 替换与认证选择收敛（2026-10-15）

当前 UI 使用 Naive UI 2.45.3，移除 Reka UI，优先库预设样式；AppDialog、表单薄封装、工作区/设置标签、首页菜单、取色与折叠、内容滚动均迁移。NConfigProvider 管理应用主题与中文文案，终端主题独立。固定页/会话标签分区保持固定入口可见，终端宿主不交给库默认卸载生命周期。

主机与快速连接只提供密码和公钥认证；旧仅 keyboardInteractive 的记录编辑时映射到密码偏好，用户保存后写主要步骤。后端动态 OTP/PAM 与旧计划兼容保持，协议枚举仍存在不等于界面提供该选项。新增依赖已核对 MIT、Node >=20、Vue ^3.0.0；禁用脚本安装，pnpm audit 返回 0 个公告。没有新增 IPC、数据库 migration 或 Rust 依赖。

本轮仅静态验证，不代表三平台效果、表单焦点/IME或 SSH 已通过运行验收；此前 Reka 与原生控件记录是历史实施路径，当前以本节与 13 文档为准。

最终静态结果：`pnpm exec vue-tsc --noEmit`、`pnpm exec prettier --check src`、`cargo fmt --all -- --check`、`git diff --check` 全部通过。源码与依赖锁无 Reka；Vue 中原生 select/button/textarea/details/summary 已移除，仅保留两个隐藏 JSON 文件选择 input。19 份 Markdown 的围栏与 139 个本地文件链接目标通过检查，未检查锚点。NSelect 显式采用 `display-directive="if"`，避免关闭后残留弹层标记阻止快捷键与终端焦点恢复；该行为仍需 WebView 运行验证。

## 9. 终端基础体验修复（2026-10-15）

选择与弹窗整理：body 默认 user-select:none（含 WebKit），保留输入、可编辑区域、终端库原有选区与显式正文复制。AppDialog 改用 NModal card 的 content-scrollable，受视口最大高度限制；所有业务弹窗的保存/取消/确认/认证按钮移至 `#footer`，NFlex 提供布局，正文滚动不携带按钮。主机/凭据/认证/确认的外置 submit 保留 form 关联；认证无输入字段时焦点可落到 footer 继续按钮。设置内嵌模式复用同一插槽。运行与视觉验收未执行。

本轮 `vue-tsc --noEmit`、Prettier 全源码检查、`git diff --check` 通过；静态扫描确认 6 个 AppDialog 调用组件均提供 footer，4 个外置提交引用均有对应表单 ID，无旧 dialog-actions 正文操作区。未构建、启动或执行 SSH/视觉验证。

按用户后续要求删除 terminal-toolbar 与相关 CSS。名称/状态沿用标签和底部状态栏，搜索、配色、断开/重连移至 SSH 标签 NDropdown 上下文菜单，支持右键及 Shift+F10/菜单键，动作绑定菜单目标标签。搜索快捷键保持；菜单关闭卸载内容以释放快捷键/焦点保护标记。源码静态通过不代表菜单键盘/焦点已通过 WebView 验收。

根据用户实际终端反馈，限制 html/body/#app 页面滚动与边缘回弹，并在 xterm 滚动元素隔离滚动链，保留组件内部滚动。terminal-stage 的 CSS 背景变量与 xterm 共用当前标签配色解析。纯 Meta/Control/Alt/Shift 按下不进入 xterm 组合输入处理，避免无实际输入时跳底，实际按键及复制动作仍走原链路。顶部工作区标签使用 Naive UI 的 tabPaddingMediumCard=10px 12px。用户已确认长按 Enter 产生空白是 zsh 行为，本次不修改 SSH 字节、重复按键或远端配置。

`pnpm exec vue-tsc --noEmit`、`pnpm exec prettier --check src` 与 `git diff --check` 静态检查通过。代理未构建、启动、连接 SSH 或进行 WebView 视觉验证；滚轮边界、选区/修饰键、IME 与配色热切换仍需用户复验。

## 10. 凭据与主机新契约（2026-10-15）

- 公共凭据只含 `password` / `privateKey`；私钥正文与可选解密密码作为单个系统凭据库项目保存，SQLite 仅存元数据。原生选择只授权导入，连接不重新读取源私钥文件。接口不返回秘密或主机专属凭据 ID。
- 主机引用公共凭据，或直接填写专属密码。非空自动保存，明确空串删除后在连接时询问；省略参数保留原值供收藏等元数据更新使用。无“记住密码”或独立保存私钥口令功能。快速连接不持久化直接输入秘密，OTP 始终动态提示。
- 凭据状态仍为 pending/ready/deleting；新项准备成功后事务绑定，失败补偿，已提交的主机变更不因旧项清理失败而报告保存失败。启动恢复遍历全部待清理项目，主机删除/替换只处理其所属秘密。
- 前端提交后清理敏感字段；主机保存成功而配色保存失败时只重试配色，避免空密码重复提交清除已保存密码。数据库操作返回规范化主机 DTO，前端直接合并。
- 按用户要求不兼容旧数据库：重写 `0001_initial.sql`、删除旧 `0002`，schemaVersion 为 1。未删除或重置用户实际数据库/系统凭据。相关设计见 [存储](07-storage.md) 与 [ADR-022](12-decisions.md#adr-022公共凭据与主机专属密码2026-10-15)。

增加/更新5项存储测试源码，覆盖凭据类型与引用、所属秘密隔离、保留/清除/替换、删除与孤立项清理、返回 DTO；按约束未编译或运行。OS keyring 实际权限、锁定/不可用、崩溃恢复和单项大小限制均待平台验证，较大的私钥在部分平台可能保存失败，禁止明文回退。

本轮最终静态检查：`pnpm exec vue-tsc --noEmit`、`pnpm exec prettier --check src`、`cargo fmt --all -- --check`、`cargo metadata --no-deps --locked --format-version 1`、`git diff --check` 通过；20份 Markdown 的代码围栏与137个本地链接目标检查通过（未校验锚点）。依赖 MSRV/许可证从本地固定 release 清单核对。未执行任何项目编译、启动、Rust 测试或 SSH 连接。

## 11. 新库启动失败修复（2026-10-15）

用户执行 `pnpm tauri dev` 已通过本机 Rust 编译，但 setup 报“数据库来自更高版本，拒绝写入”。原因是新 schema 已重置为 1，桌面壳仍使用包含旧版本 2 的 `termvexa.sqlite3`。本轮将壳入口改为独立 `termvexa-credentials-v1.sqlite3`；模型代际与内部 migration 版本分别管理，不再打开旧库。旧数据库及 WAL/SHM、备份、OS keyring 原样保留，新库首次为空，需重新配置主机/凭据/设置/信任。版本拒绝与 checksum 校验继续生效，未实现旧库转换。

静态 `cargo fmt --all -- --check`、`git diff --check` 通过，并检查新路径在桌面入口与存储文档一致。用户日志仅证明修复前版本在 macOS 编译成功；代理未执行编译、启动或 SSH 验证，修复后启动待复验。

## 12. 本地终端字体资源（2026-10-15）

按用户列表下载12个家族、35个原始字体文件，共9,511,856字节；连同许可证、版权、manifest 与说明约9.6 MB。来源与版本见 [资源说明](../public/fonts/README.md)，每个字型记录 SHA-256。资源放在 public/fonts，确保字体与许可证随包复制；CSS 仅引用本地 URL。设置采用既有 Naive UI 下拉，默认 JetBrains Mono，旧自由名称读取归一，保存 Rust 白名单；字体就绪后更新既有 xterm 并重算尺寸，迟到回调检查当前选择与销毁状态。无新增 npm/Rust 依赖、IPC 命令或数据库迁移。

静态检查通过：`pnpm exec vue-tsc --noEmit`、`pnpm exec prettier --check src public/fonts/manifest.json`、`cargo fmt --all -- --check`、`git diff --check`。fontTools 读取全部35份字体表，校验 SHA-256、ASCII覆盖与等宽；CSS本地文件路径、许可证非空、Rust/TS/manifest 的12项名单核对通过。21份 Markdown 的围栏与144个本地链接目标检查通过（不含锚点）。未编译、打包、启动应用或连接SSH；字体加载/合成样式/中文回退/快速切换/实际许可证入包与三平台渲染仍待运行验收。

配色选项分行（2026-10-15）：ThemeSettings 以 NFlex vertical 将名称与色块分为上下两行，按钮自适应高度；色块不参与可访问名称。前端类型/格式与差异静态检查通过，未构建/启动或进行 WebView 视觉验收。

字体下拉实际字型预览（2026-10-15）：SettingsDialog 使用 NSelect render-label，经 AppSelect 透传，选项与当前选中名称均绑定对应 Termvexa 字体别名；浏览器按可见标签加载本地字型，不预加载全部家族。类型、格式、差异静态检查通过；离线字体加载、下拉弹层与三平台视觉效果未运行验证。

## 13. 提取配色替换项目预设（2026-10-15）

用户提供解包目录后，以 Nord Dark 定位四个压缩 JS 包中的同一配色数组，截取21,621字节进行 TypeScript AST 静态解析，取得64套方案，四处数组内容完全一致。仅提取名称、16色 ANSI、前背景/光标/选区，不执行解包应用代码。映射与源文件 SHA-256 见 [配色资源](../resources/README.md)。移除原8套自有预设，默认 Nord Dark；前端与 Rust 共享资源名单，终端临时引用的有效性检查同步使用该名单。旧预设明暗引用读取归一为 Nord Dark/Light，Ocean 保留 ID 使用提取颜色，用户自定义配色保留；导入缺项及删除默认方案回退基准统一为 Nord Dark。

无新增依赖、IPC命令或数据库迁移。验证仅为源码与静态资源检查；应用启动、实际 xterm 色值/选区效果、三平台 WebView 验收仍未执行。最终静态检查通过：`pnpm exec vue-tsc --noEmit`、`pnpm exec prettier --check src scripts/extract-terminal-palettes.mjs resources`、`cargo fmt --all -- --check`、`git diff --check`。独立重提取结果与资源一致，64项 ID 唯一、原终端色值与 ANSI 顺序逐项一致、4个源文件摘要匹配；22份 Markdown 围栏与151个本地链接目标检查通过（不含锚点）。未编译或运行应用。

## 14. 首页与设置滚动区域整理（2026-10-15）

配色侧栏列表接入 NScrollbar，限制 min(480px,48dvh) 高度并持续显示轨道，搜索与导入置于列表外。首页移除 home-brand/home-navigation-footer，设置和凭据移除 eyebrow，三页统一主机页 NH1/NText 与28px间距/1540px宽度。App 首页内容改为受限高度的页面壳，主机与凭据分别滚动；AppDialog 内嵌模式的正文用 NScrollbar，#footer 独立占据底部，设置按钮始终可见。弹窗仍使用 NModal 的库 footer；终端宿主结构和保活逻辑未改。

前端类型、格式及差异静态检查通过；无依赖、IPC或存储变更。未编译、启动应用或执行视觉验收，待验证小视口、长列表、各设置页和顶部/左侧标签布局下的操作区可见性。

## 15. 配色网格布局（2026-10-15）

根据用户最新布局要求，方案卡片改为 NGrid 按容器宽度1–5列平铺，网格按最新要求自然展开，不限高、不设独立滚动，随设置正文统一滚动。点击卡片设置全局配色草稿并显示选中标记；自定义编辑默认折叠于网格下方，保存为自定义在编辑区最下方，全局保存设置继续留在固定底部。预览首次展开初始化，收起保留实例；保存/导入后选中新方案，移除已处理导入标记；撤销设置时重新同步编辑器基准。

无依赖/IPC/存储变更。前端类型、格式与差异静态检查通过；未构建、启动或进行视觉验收。

首页显隐回归修复（2026-10-15）：用户反馈切换凭据/设置时内容区仍显示主机。上次拆分页面滚动区将 v-show 挂在 AppScrollArea，最终根为 NScrollbar 的 ResizeObserver 包装，指令缺少稳定元素承接。现增加原生主机页容器控制 v-show，滚动组件内部保活，避免内容叠加。前端类型、格式及差异静态检查通过；未启动应用，需复验主机→凭据→设置→主机与顶部/左侧布局切换。

配色网格滚动调整（2026-10-15）：移除网格外的 AppScrollArea 与 scheme-list 限高样式，全部卡片自然展开，随设置正文统一滚动；底部设置操作区继续固定。vue-tsc --noEmit、Prettier 与 git diff --check 静态检查通过；未构建、启动或进行视觉验收。

Pro 粗体颜色修复（2026-10-15）：TerminalRegistry 与 ThemeSettings 的 xterm 初始化均显式设置 drawBoldTextInBrightColors=false，粗体不再自动提升普通 ANSI 色；显式高亮色及原配色数据保留。vue-tsc --noEmit、Prettier 与 git diff --check 静态检查通过；未构建、启动或连接主机，实际颜色需按验证文档复验。

工作区标签重构（2026-10-15）：App.vue 不再使用 NTabs/NTab，新增 WorkspaceTabs 以原生按钮、稳定 terminalId、类型化 activate/focusTerminal/close/menu/create 事件实现顶部与侧栏标签。固定首页/SFTP 与可滚动会话区共用单一 tablist；包含选中项滚动显露、方向键/Home/End/Enter/空格、Delete 关闭、右键/菜单键/Shift+F10、关闭后焦点衔接与面板 ARIA 关联。NDropdown 菜单、关闭确认及 TerminalRegistry 保活不变；没有实现拖拽排序或新增持久化配置。vue-tsc --noEmit、修改源码的 Prettier 与 git diff --check 静态检查通过；未构建、启动或连接主机，布局、焦点及终端保活待运行复验。

工作区标签悬停调整（2026-10-15）：整项 workspace-tab-item 承接 hover 背景，标题和关闭按钮透明并继承文字色；固定页按钮保持完整高亮，键盘焦点轮廓保留。Prettier 与 git diff --check 静态检查通过；未构建或启动应用，视觉效果待复验。

工作区标签居中与分隔（2026-10-15）：横向标签栏、标签列表和动态区均居中，动态标签四角6px圆角；首页/SFTP固定区右侧加竖分隔线，侧栏模式改为底部分隔线，保留整项悬停高亮。Prettier 与 git diff --check 静态检查通过；未构建或启动应用，溢出滚动与视觉效果待复验。

标签激活色调整（2026-10-15）：WorkspaceTabs 使用 useThemeVars 将 Naive UI buttonColor2/buttonColor2Hover/textColor1 注入局部 CSS 变量，激活底色强于悬停，整项高亮与四角圆角保留。类型、格式及差异静态检查通过；未构建或启动应用，浅深主题视觉效果待复验。

标签关闭图标（2026-10-15）：以 NIcon/SVG 替代关闭字符，14px图标在24px按钮内居中；悬停使用 Naive UI closeColorHover 半透明层，整项 hover/active 保留。类型、格式与差异静态检查通过；未构建或启动应用，实际视觉待复验。

标签尺寸与图标居中（2026-10-15）：36px标签搭配24px关闭按钮，上下/右侧留白均6px；关闭按钮 padding/margin 清零，flex 对齐并将 NIcon/SVG 块级显示。固定/动态标签及关闭按钮圆角绑定主题 borderRadius。类型、格式与差异静态检查通过；未构建或启动应用，主题调整和实际像素对齐待视觉复验。

重复标签与新建按钮（2026-10-15）：WorkspaceStore 按保存主机ID或快速连接端点/用户维护临时递增计数，不保存秘密，中途关闭不重排编号，最后一个同连接标签关闭后清除计数。WorkspaceTabs 新增弹性留白，CSS 将标签列表及会话区改为 flex:0 1 auto，新建按钮36×36不收缩，连接数位于末端；侧栏沿纵向收缩滚动，新建按钮靠左。类型、格式与差异静态检查通过；未构建、启动或连接主机，尺寸/溢出与编号行为待运行验收。

macOS Option 跳词（2026-10-15）：终端自定义键处理在命令路由之后识别无额外修饰键的 Option+左右键，阻止默认方向序列并调用 term.input(ESC b/f, true)，继续经过 onData 与有界输入队列。保留 IME/AltGraph及模态检查，不开启 macOptionIsMeta，不改变 Option 字符输入或其他平台。vue-tsc、Prettier 与差异静态检查通过；未构建、启动或连接远端，实际 Shell 行编辑跳词待复验。

链接激活残留拖选（2026-10-15）：静态核对 xterm 6 Linkifier 的 mouseup 激活与 SelectionService 的 document mouseup 清理路径，定位业务 stopPropagation 截断清理。共享 activate 回调仅 preventDefault，保留冒泡，并以 setTimeout(0) 延后调用 opener，已释放终端取消延迟动作。vue-tsc、Prettier 与差异静态检查通过；未构建、启动或打开真实链接，回窗后鼠标状态待运行复验。

选区透明蒙色（2026-10-15）：新增独立 terminal/theme.ts，以 applyTerminalTheme 统一实际终端及预览主题应用；无显式 selectionForeground 时，将原生选区底色临时设透明，CSS 按原色在文字上方绘制 xterm 自己管理的矩形，防止背景重复混合；显式选区文字色使用原生渲染。模块含中文边界注释，不改依赖、不添加逐字遍历或鼠标/渲染事件监听、不修改持久化数据。固定 xterm 6 DOM 图层为适配边界；类型、格式及差异静态检查通过，未构建、启动或执行视觉验收，实际合成与搜索/光标/列选区待复验。

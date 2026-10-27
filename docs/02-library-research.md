# 依赖选型与升级约束

更新：2026-10-27。版本和 features 以 manifest、Cargo.lock、pnpm-lock.yaml 为准。本文保留选型依据、许可和升级约束。

## 1. 主要依赖

| 领域 | 锁定基线与来源 | 用途与约束 |
| --- | --- | --- |
| 桌面 | Tauri 2.11.5 / build 2.6.3 / JS API 2.11.0；[Channel](https://v2.tauri.app/reference/javascript/api/namespacecore/)、[Capabilities](https://v2.tauri.app/security/capabilities/) | Commands + 原始二进制 Channel；自定义命令须 AppManifest/ACL 与资源 owner 双重校验 |
| 前端 | Vue 3、TypeScript、Pinia 4.1.0、Naive UI 2.45.3、@tabler/icons-vue 3.49.0 | 通用控件和轻量状态，桌面行为见 [交互](09-desktop-ux.md) |
| 终端 | xterm 6.0.0 / Fit 0.11.0 / Search 0.16.0 / Serialize 0.14.0 / WebLinks 0.12.0；[API](https://xtermjs.org/docs/api/terminal/classes/terminal/)、[流控](https://xtermjs.org/docs/guides/flowcontrol/)、[安全](https://xtermjs.org/docs/guides/security/) | 原始 Uint8Array；解析回调用于 ACK，不等于 GPU 呈现。无需 addon-attach/WebGL |
| SSH | [russh 0.64.1](https://docs.rs/russh/0.64.1/russh/)，禁用默认 features，启用 ring/rsa | Apache-2.0，声明 MSRV 1.89；认证/PTY 请求须等待服务器接受，不能将发送成功视为完成 |
| SFTP | [russh-sftp 3.0.1](https://docs.rs/russh-sftp/3.0.1/russh_sftp/)，禁用默认 features | Apache-2.0，未声明 MSRV；通过 AsyncRead/AsyncWrite 接 SSH subsystem，增量浏览避免全量目录收集 |
| 数据库 | [SQLx 0.8.6](https://docs.rs/sqlx/0.8.6/sqlx/)，sqlite/runtime-tokio/migrate/macros | MIT OR Apache-2.0；Rust 独占 SQL。迁移事务和父表重建见 [存储](07-storage.md) |
| SQLCipher | [libsqlite3-sys 0.30.1](https://docs.rs/crate/libsqlite3-sys/0.30.1/source/)，bundled-sqlcipher-vendored-openssl | 内嵌 SQLCipher 4.5.7（BSD）与 OpenSSL；不回退普通 SQLite，依赖目标平台原生工具链 |
| 主密钥/备份加密 | [Argon2 0.6.0](https://docs.rs/argon2/0.6.0/argon2/) alloc/zeroize；[AES-GCM 0.11.1](https://docs.rs/aes-gcm/0.11.1/aes_gcm/) zeroize | MIT OR Apache-2.0，声明 MSRV 1.85；用于主密钥校验与第三方独立密码云包，不替代 SQLCipher 页加密 |
| OS 缓存 | [keyring 3.6.3](https://docs.rs/keyring/3.6.3/keyring/)，按平台启用 backend | MIT OR Apache-2.0，声明 MSRV 1.75；macOS apple-native、Windows windows-native、Linux sync-secret-service/crypto-rust/vendored，仅缓存主密钥 |
| Local | [portable-pty 0.9.0](https://crates.io/crates/portable-pty/0.9.0) | MIT，未声明 MSRV；Unix PTY/Windows ConPTY、独立阻塞 I/O，完整进程树回收待完善 |
| Serial | [serialport 4.10.1](https://docs.rs/serialport/4.10.1/serialport/)，禁用默认 libudev feature | MPL-2.0，声明 MSRV 1.59；None/Software/Hardware 流控及平台独占 |
| 文本编码 | [encoding_rs 0.8.42](https://docs.rs/encoding_rs/0.8.42/encoding_rs/) | (Apache-2.0 OR MIT) AND BSD-3-Clause，声明 MSRV 1.88；流式解码，不可表示输入明确失败 |
| Telnet | 内部 terminal-telnet + Tokio 1.50.0 | 不调用系统 telnet；实现 [RFC 854](https://www.rfc-editor.org/rfc/rfc854) 及 BINARY/ECHO/SGA/TTYPE/NAWS 的有界子集 |
| HTTPS | [reqwest 0.13.5](https://docs.rs/reqwest/0.13.5/reqwest/)，禁用默认 features，启用 rustls/json | MIT OR Apache-2.0，声明 MSRV 1.85；云访问验证证书、禁重定向及自动重试、有界读响应 |
| WebDAV | [quick-xml 0.42.0](https://docs.rs/quick-xml/0.42.0/quick_xml/)，禁用默认 features | MIT，声明 MSRV 1.86；受限 UTF-8 DAV 解析，不启用外部实体 |
| 备份压缩/封装 | [zstd 0.13.3](https://docs.rs/zstd/0.13.3/zstd/)（禁用默认 features）、base64 0.22.1 | MIT / MIT OR Apache-2.0，声明 MSRV 1.64 / 1.48；先压缩再加密，解压窗口和输出受限 |

SerializeAddon 0.14.0（MIT，未声明独立 engines/features/MSRV）与 xterm 6.0.0 使用同一源码基线。使用固定 release 的 `serialize({excludeModes,excludeAltBuffer})` 保留缓冲样式，不使用 HTML 导出；插件标为 experimental，升级须验证复杂样式、宽字符及备用屏往返。来源：[包元数据](https://registry.npmjs.org/@xterm/addon-serialize/0.14.0)、[固定提交 API](https://github.com/xtermjs/xterm.js/blob/f447274f430fd22513f6adbf9862d19524471c04/addons/addon-serialize/typings/addon-serialize.d.ts)。

私钥格式与公钥查看对齐 russh 0.64.1 的 `decode_secret_key`（OpenSSH、RSA PEM、PKCS#8/EC 标记、PuTTY PPK）。OpenSSH 使用重导出的 `PrivateKey::from_openssh` 直接读取公开部分，不解密；其余格式复用认证解析器，必要时以保存或本次输入的口令解密。公钥通过 `public_key` / `PublicKey::to_openssh` 编码，锁内实现为 ssh-key 0.7.0-rc.11。具体算法与加密方式仍受该 release 限制，不根据格式名称承诺全变种互通。

基础库共用 Tokio/async-trait/serde/uuid/bytes/zeroize 等锁定依赖；`spawn_blocking` 用于短期阻塞工作，长期 PTY/设备 I/O 使用独立线程。表中 MSRV 是直接依赖声明，不是整个传递集合的兼容保证；SQLCipher/OpenSSL、Zstd、rustls 所选 aws-lc 等还涉及原生编译和许可集合。

`@tabler/icons-vue` 固定 3.49.0，MIT；该 release 未声明 Node engines，Vue peer 范围为 `>=3.0.1`，组件提供 `size`、`color`、`stroke`、`title` 属性，并由 `@tabler/icons` 同版本依赖提供图形。来源：[固定包元数据](https://registry.npmjs.org/@tabler/icons-vue/3.49.0)、[该 release 的 Vue 用法](https://github.com/tabler/tabler-icons/blob/v3.49.0/packages/icons-vue/README.md)、[MIT 许可](https://github.com/tabler/tabler-icons/blob/v3.49.0/LICENSE)。

## 2. 平台适配与资源

macOS 文件承诺与原生菜单使用 objc2 0.6.5、block2 0.6.2、objc2-app-kit/foundation 0.3.2，仅 macOS target 按需启用 features。objc2/foundation/block2 为 MIT，AppKit 为 Zlib OR Apache-2.0 OR MIT，声明 MSRV 1.71。AppKit 对象及通知令牌在主线程管理，文件承诺后台兑现；API 来源见 [NSFilePromiseProvider](https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/struct.NSFilePromiseProvider.html) 和 [菜单](https://github.com/tauri-apps/muda/issues/235)。串口角色枚举另用 core-foundation 0.10.1、io-kit-sys 0.4.1（MIT/Apache-2.0）。

Windows 文件/目录拖出使用固定 [windows 0.62.2](https://docs.rs/crate/windows/0.62.2/source/) 与 [windows-core 0.62.2](https://docs.rs/crate/windows-core/0.62.2/source/)（MIT OR Apache-2.0，声明 MSRV 1.82），按目标启用 Shell/OLE、COM/存储、内存、文件属性、事件与窗口消息 API。自行实现目录描述符、只读内容流及异步生命周期，不依赖 `virtual-file-transfer` 或预下载临时文件；文件内容读取留在 Rust，STA 等待通过 COM 消息泵分派窗口/COM 调用。API 依据为 [Shell 数据格式](https://learn.microsoft.com/en-us/windows/win32/shell/clipboard)、[FILEDESCRIPTORW](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-filedescriptorw)、[异步数据传输](https://learn.microsoft.com/en-us/windows/win32/shell/datascenarios#dragging-and-dropping-shell-objects-asynchronously) 和 [CoWaitForMultipleHandles](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cowaitformultiplehandles)。系统后台热键尚未引入，见 [路线](10-roadmap.md)。

Linux 拖出直接固定使用已在 Tauri/Wry 锁图中的 [gtk 0.18.2](https://docs.rs/crate/gtk/0.18.2/source/)（MIT，声明 MSRV 1.70），仅 Linux target 使用默认 features，不启用额外版本特性；复用其 gdk 0.18.2 / glib 0.18.5 重导出，未引入第二套 GTK 版本。原生拖动使用 `drag_begin_with_coordinates`、`drag-data-get/end/failed` 与 `SelectionData::set_uris`，URI 编码复用 url 2.5.8。gtk3-rs 已停止维护，既有 glib 0.18.5 公告及其他桌面传递风险仍见 [风险记录](#4-已知依赖风险)，直接引用不代表公告已修复。实现选择 URI 私有暂存的原因、保留期、清理及最终复制边界见 [SFTP](16-sftp-workspace.md#4-下载与原生拖出)。

12 家族字体的固定版本、来源、许可和摘要集中在 [字体清单](../resources/fonts/README.md)；配色、拼音表和用户指定 SSH 系统图标集中在 [资源来源](../resources/README.md)。不重复维护尺寸、提取过程或组件样式。

品牌资源生成使用已锁定的 Tauri CLI 2.12.3，以及独立 Python ≥3.10 工具环境中的 Pillow 12.3.0、resvg-py 0.5.0，版本固定在 [图标工具依赖](../scripts/icon-requirements.txt)。按这些 release 的 `svg_to_bytes`、PNG 保存、ICO/ICNS 解码接口生成和静态校验，工具库不进入应用运行依赖，也不改变 Cargo/pnpm 锁文件。复现入口与平台留白、ICO 顺序约束见 [品牌资源](../resources/README.md#桌面应用品牌图标)。

## 3. 升级时必须复核的边界

- 日志颜色快照集中在 `session-logs/snapshot.ts`，读取固定 xterm 6 的 `_core._themeService.colors` 以保留运行中动态调色，结构不兼容时阻止归档而非静默替换颜色；升级须复核此适配和 SerializeAddon。
- xterm 内部用户输入标记与鼠标动作编码集中在 inputOrigin.ts，选区适配也依赖渲染结构。升级须复核 IME、自动应答、广播与选区，不能仅依据公开 onData 判断用户输入。
- Tauri 原始 Channel、ACK、后台标签、取消和三种 WebView 的内存水位须联合验证；AppManifest、capability 与 owner 不可只保留其一。
- SQLCipher raw key、加密副本 rekey、serialize/deserialize、迁移/外键切换和故障恢复须使用实际数据库验证，不能由 API 存在推定可靠。
- keyring 后端需覆盖服务缺失/锁定/缓存失败，不允许明文或 mock 回退；PTY/串口需验证阻塞时唤醒与平台清理。
- 依赖 release、features、传递 MSRV、原生工具链、许可证及公告须与锁文件一起复核。已知公告集中在 [风险记录](#4-已知依赖风险)。

## 4. 已知依赖风险

当前依赖风险与处理方向如下：

| 依赖 | 公告 | 当前处理边界 |
| --- | --- | --- |
| rsa 0.9.10 / 0.10.0-rc.18 | [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html) | RSA 时序侧信道；实际 SSH 签名路径未启用 blinding，不能以未暴露解密 oracle 判为安全。0.9.10 在锁内可选 SQLx/MySQL 分支，不代表实际启用 |
| glib 0.18.5 | [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) | Linux GTK3 传递依赖不安全，需升级依赖或审阅补丁 |
| proc-macro-error 1.0.4 | [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) | 上游停止维护，跟踪桌面依赖升级 |
| unic 系列 | RUSTSEC-2025-0081/0075/0080/0100/0098 | 停止维护，需跟踪替代；与可利用漏洞分开评估 |

升级或发布时同步复核实际启用依赖、MSRV、原生工具链、notices 与资源许可。

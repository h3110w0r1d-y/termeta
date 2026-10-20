# 依赖选型与核查边界

更新：2026-10-20。版本和 features 以 manifest、Cargo.lock、pnpm-lock.yaml 为准。本文保留选型依据与升级注意事项，不记录逐轮调研/API 排查过程；静态核对不代表编译、兼容性或安全审计通过。

## 1. 主要依赖

| 领域 | 锁定基线与来源 | 用途与约束 |
| --- | --- | --- |
| 桌面 | Tauri 2.11.5 / build 2.6.3 / JS API 2.11.0；[Channel](https://v2.tauri.app/reference/javascript/api/namespacecore/)、[Capabilities](https://v2.tauri.app/security/capabilities/) | Commands + 原始二进制 Channel；自定义命令须 AppManifest/ACL 与资源 owner 双重校验 |
| 协议回跳 | [tauri-plugin-deep-link 2.4.10](https://crates.io/crates/tauri-plugin-deep-link/2.4.10)，复用 single-instance 2.4.2，均不新增 feature | Apache-2.0 OR MIT，deep-link 声明 MSRV 1.77.2；desktop scheme 配置及官方 register，Rust 原生接收，WebView 不读回跳 URI |
| 前端 | Vue 3、TypeScript、Pinia 4.1.0、Naive UI 2.45.3、@tabler/icons-vue 3.49.0 | 通用控件和轻量状态，桌面行为见 [交互](09-desktop-ux.md) |
| 终端 | xterm 6.0.0 / Fit 0.11.0 / Search 0.16.0 / Serialize 0.14.0 / WebLinks 0.12.0；[API](https://xtermjs.org/docs/api/terminal/classes/terminal/)、[流控](https://xtermjs.org/docs/guides/flowcontrol/)、[安全](https://xtermjs.org/docs/guides/security/) | 原始 Uint8Array；解析回调用于 ACK，不等于 GPU 呈现。无需 addon-attach/WebGL |
| SSH | [russh 0.64.1](https://docs.rs/russh/0.64.1/russh/)，禁用默认 features，启用 ring/rsa | Apache-2.0，声明 MSRV 1.89；认证/PTY 请求须等待服务器接受，不能将发送成功视为完成 |
| SFTP | [russh-sftp 3.0.1](https://docs.rs/russh-sftp/3.0.1/russh_sftp/)，禁用默认 features | Apache-2.0，未声明 MSRV；通过 AsyncRead/AsyncWrite 接 SSH subsystem，增量浏览避免全量目录收集 |
| 数据库 | [SQLx 0.8.6](https://docs.rs/sqlx/0.8.6/sqlx/)，sqlite/runtime-tokio/migrate/macros | MIT OR Apache-2.0；Rust 独占 SQL。迁移事务和父表重建见 [存储](07-storage.md) |
| SQLCipher | [libsqlite3-sys 0.30.1](https://docs.rs/crate/libsqlite3-sys/0.30.1/source/)，bundled-sqlcipher-vendored-openssl | 内嵌 SQLCipher 4.5.7（BSD）与 OpenSSL；不回退普通 SQLite，三平台原生工具链待验证 |
| 主密钥/备份加密 | [Argon2 0.6.0](https://docs.rs/argon2/0.6.0/argon2/) alloc/zeroize；[AES-GCM 0.11.1](https://docs.rs/aes-gcm/0.11.1/aes_gcm/) zeroize | MIT OR Apache-2.0，声明 MSRV 1.85；用于主密钥校验、官方同主密钥云包与第三方独立密码云包，不替代 SQLCipher 页加密 |
| OS 缓存 | [keyring 3.6.3](https://docs.rs/keyring/3.6.3/keyring/)，按平台启用 backend | MIT OR Apache-2.0，声明 MSRV 1.75；macOS apple-native、Windows windows-native、Linux sync-secret-service/crypto-rust/vendored，仅缓存主密钥 |
| Local | [portable-pty 0.9.0](https://crates.io/crates/portable-pty/0.9.0) | MIT，未声明 MSRV；Unix PTY/Windows ConPTY、独立阻塞 I/O，完整进程树回收待完善 |
| Serial | [serialport 4.10.1](https://docs.rs/serialport/4.10.1/serialport/)，禁用默认 libudev feature | MPL-2.0，声明 MSRV 1.59；None/Software/Hardware 流控及平台独占 |
| 文本编码 | [encoding_rs 0.8.42](https://docs.rs/encoding_rs/0.8.42/encoding_rs/) | (Apache-2.0 OR MIT) AND BSD-3-Clause，声明 MSRV 1.88；流式解码，不可表示输入明确失败 |
| Telnet | 内部 terminal-telnet + Tokio 1.50.0 | 不调用系统 telnet；实现 [RFC 854](https://www.rfc-editor.org/rfc/rfc854) 及 BINARY/ECHO/SGA/TTYPE/NAWS 的有界子集 |
| HTTPS | [reqwest 0.13.5](https://docs.rs/reqwest/0.13.5/reqwest/)，禁用默认 features，启用 rustls/json | MIT OR Apache-2.0，声明 MSRV 1.85；云访问验证证书、禁重定向及自动重试、有界读响应 |
| WebDAV | [quick-xml 0.42.0](https://docs.rs/quick-xml/0.42.0/quick_xml/)，禁用默认 features | MIT，声明 MSRV 1.86；受限 UTF-8 DAV 解析，不启用外部实体 |
| 备份压缩/封装 | [zstd 0.13.3](https://docs.rs/zstd/0.13.3/zstd/)（禁用默认 features）、base64 0.22.1 | MIT / MIT OR Apache-2.0，声明 MSRV 1.64 / 1.48；先压缩再加密，解压窗口和输出受限 |

SerializeAddon 0.14.0（MIT，未声明独立 engines/features/MSRV）与 xterm 6.0.0 使用同一源码基线。使用固定 release 的 `serialize({excludeModes,excludeAltBuffer})` 保留缓冲样式，不使用 HTML 导出；插件标为 experimental，升级须验证复杂样式、宽字符及备用屏往返。来源：[包元数据](https://registry.npmjs.org/@xterm/addon-serialize/0.14.0)、[固定提交 API](https://github.com/xtermjs/xterm.js/blob/f447274f430fd22513f6adbf9862d19524471c04/addons/addon-serialize/typings/addon-serialize.d.ts)。

基础库共用 Tokio/async-trait/serde/uuid/bytes/zeroize 等锁定依赖；`spawn_blocking` 用于短期阻塞工作，长期 PTY/设备 I/O 使用独立线程。表中 MSRV 是直接依赖声明，不是整个传递集合的兼容保证；SQLCipher/OpenSSL、Zstd、rustls 所选 aws-lc 等还涉及原生编译和许可集合。

`@tabler/icons-vue` 固定 3.49.0，MIT；该 release 未声明 Node engines，Vue peer 范围为 `>=3.0.1`，组件提供 `size`、`color`、`stroke`、`title` 属性，并由 `@tabler/icons` 同版本依赖提供图形。来源：[固定包元数据](https://registry.npmjs.org/@tabler/icons-vue/3.49.0)、[该 release 的 Vue 用法](https://github.com/tabler/tabler-icons/blob/v3.49.0/packages/icons-vue/README.md)、[MIT 许可](https://github.com/tabler/tabler-icons/blob/v3.49.0/LICENSE)。

## 2. 平台适配与资源

macOS 文件承诺与原生菜单使用 objc2 0.6.5、block2 0.6.2、objc2-app-kit/foundation 0.3.2，仅 macOS target 按需启用 features。objc2/foundation/block2 为 MIT，AppKit 为 Zlib OR Apache-2.0 OR MIT，声明 MSRV 1.71。AppKit 对象及通知令牌在主线程管理，文件承诺后台兑现；API 来源见 [NSFilePromiseProvider](https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/struct.NSFilePromiseProvider.html) 和 [菜单](https://github.com/tauri-apps/muda/issues/235)。串口角色枚举另用 core-foundation 0.10.1、io-kit-sys 0.4.1（MIT/Apache-2.0）。

Windows/Linux 原生文件拖出没有落地，不将候选库写成已支持。系统后台热键同样未引入，见 [路线](10-roadmap.md)。

12 家族字体的固定版本、来源、许可和摘要集中在 [字体清单](../resources/fonts/README.md)；配色、拼音表和用户指定 SSH 系统图标集中在 [资源来源](../resources/README.md)。资源来源记录不等同发布许可审查通过，不重复维护尺寸、提取过程或组件样式。

## 3. 升级时必须复核的边界

- 日志颜色快照集中在 `session-logs/snapshot.ts`，读取固定 xterm 6 的 `_core._themeService.colors` 以保留运行中动态调色，结构不兼容时阻止归档而非静默替换颜色；升级须复核此适配和 SerializeAddon。
- xterm 内部用户输入标记与鼠标动作编码集中在 inputOrigin.ts，选区适配也依赖渲染结构。升级须复核 IME、自动应答、广播与选区，不能仅依据公开 onData 判断用户输入。
- Tauri 原始 Channel、ACK、后台标签、取消和三种 WebView 的内存水位须联合验证；AppManifest、capability 与 owner 不可只保留其一。
- SQLCipher raw key、加密副本 rekey、serialize/deserialize、迁移/外键切换和故障恢复须使用实际数据库验证，不能由 API 存在推定可靠。
- keyring 后端需覆盖服务缺失/锁定/缓存失败，不允许明文或 mock 回退；PTY/串口需验证阻塞时唤醒与平台清理。
- 依赖 release、features、传递 MSRV、原生工具链、许可证及公告须与锁文件一起复核。当前已知公告集中在 [风险记录](01-product-requirements.md#10-依赖与公告核查)，不宣称全依赖审计通过。

真实服务、设备、原生拖放/菜单及三平台验证门槛统一见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。

## 4. 账号与独立项目依赖

桌面账号复用既有 reqwest/opener，不引入 JavaScript HTTP/OAuth SDK。以下直接依赖的版本、feature 和 API 依据保留用于升级复核：

| 依赖 | 用途与核查 |
| --- | --- |
| [base64 0.22.1](https://crates.io/crates/base64/0.22.1)、[sha2 0.10.9](https://crates.io/crates/sha2/0.10.9)、[rand_core 0.6.4](https://crates.io/crates/rand_core/0.6.4)（getrandom） | `URL_SAFE_NO_PAD`、`Sha256::digest`、`OsRng.try_fill_bytes` 用于随机 proof 与 S256；MIT OR Apache-2.0，base64 MSRV 1.48，其余未声明 |
| [chrono 0.4.45](https://docs.rs/chrono/0.4.45/chrono/)（禁默认 feature，仅 std） | RFC3339/DateTime 与 SystemTime 转换，再换算为 Instant；MIT OR Apache-2.0，MSRV 1.62.0 |

协议回跳固定 deep-link 2.4.10 与 single-instance 2.4.2。deep-link 2.4.10 依赖 Tauri ^2.10；2026-10-19 核查的 2.5/2.6 release 要求 ^2.12，维持当前桌面框架时不直接升级。仅复用原生注册/Opened/单实例参数，禁止将 URI 全局广播到 WebView，无需 JS 插件或新增 WebView 权限。升级须复核 Plugin.initialize、运行期事件与原生参数语义。来源：[固定 deep-link 源码](https://docs.rs/crate/tauri-plugin-deep-link/2.4.10/source/)、[single-instance 源码](https://docs.rs/crate/tauri-plugin-single-instance/2.4.2/source/)、[官方协议说明](https://v2.tauri.app/plugin/deep-linking/)。

历史公告查询：2026-10-18 SerializeAddon 的 npm 查询、2026-10-19 deep-link 新增传递集合及 chrono 的 OSV 查询未返回公告。它们只代表当时指定版本的局部结果，不是实时或完整审计；未关闭风险仍见 [需求风险表](01-product-requirements.md#10-依赖与公告核查)。

服务端、用户端、管理端分别维护依赖和锁文件，见 服务端依赖、用户端依赖、管理端依赖。这些不计作桌面依赖变化；客户端 updater 插件仍待单独接入和 release/权限核查。

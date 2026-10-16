# Termvexa

基于 Tauri 2、Rust、Vue 3 和 TypeScript 的桌面终端客户端，目标平台为 Windows、macOS、Linux。

当前源码提供 SSH、Telnet、基础本地 PTY 与串口文本终端，以及主机/分组/凭据管理、SQLCipher 加密保险库、多标签与分屏广播、显式工作区保存、SFTP 和 SSH 端口转发。**尚未完成当前版本的完整集成、三平台与发布验收**；已有 macOS 使用反馈及局部串口验证不代表整体通过。

| 能力         | 当前范围                                                                                                              |
| ------------ | --------------------------------------------------------------------------------------------------------------------- |
| 主机与保险箱 | 保存 SSH/Telnet/串口及工作区，分组、收藏、标签、拼音搜索、批量打开；主机信任与最近20次连接历史                        |
| 终端         | SSH 密码/公钥/动态 OTP、信任校验；Telnet 终端内明文登录；本地系统 Shell；串口参数与字符集转换；断线保留原缓冲并可重连 |
| 工作区       | 顶部/左侧标签、排序、嵌套分屏、窗格迁移、比例调整/区域交换、输入广播；保存布局与已保存连接引用，手动打开新会话        |
| SFTP         | 单 SSH 终端右侧栏、上传下载/覆盖确认/目录合并；固定双远端工作区及栏间对拷；原生拖入和 macOS 文件/目录拖出             |
| 端口转发     | 本地、远程、动态 SOCKS5；引用保存 SSH 主机，独立连接、手动启停，切页保活                                              |
| 设置         | 独立窗口、自动保存与跨窗口热应用、应用明暗/主色、快捷键、12种本地字体、64套终端配色与 JSON 导入导出                   |

尚未实现的主要范围包括 Windows/Linux 原生拖出、本地文件浏览、通用连接租约与完整传输管理、ProxyJump/Agent、串口手动 DTR/RTS 与 HEX、本地自定义启动配置、打开标签自动恢复及签名发布。详细状态见 [需求与实现状态](docs/01-product-requirements.md#8-当前实现与限制)。

## 代码与文档

- `crates/terminal-core`：协议无关模型、actor、取消与有界数据流。
- `crates/terminal-ssh`、`terminal-telnet`、`terminal-local`、`terminal-serial`：协议/设备适配；SSH crate 同时负责转发。
- `crates/terminal-storage`、`terminal-security`：SQLCipher、迁移、Argon2id 与 OS 主密钥缓存。
- `crates/terminal-files`、`terminal-sftp`：文件端口、有界传输与 SFTP。
- `src-tauri/src`：命令校验、窗口所有权、原生交互与依赖装配。
- `src/modules`：业务页面与状态；`src/services/desktop` 为唯一 IPC 门面。

开发前阅读 [协作规则](AGENTS.md)、[文档导航](docs/README.md) 和 [开发路线](docs/10-roadmap.md)。数据库为应用数据目录下的 `termvexa-encrypted-v2.sqlite3`，当前 schema 6，可原位升级同加密格式的 schema 2–5。备份须同时保留配套 vault 元数据；不迁移早期旧格式。详见 [存储设计](docs/07-storage.md)。

## 静态检查

```sh
pnpm install --ignore-scripts
pnpm check:types
pnpm check:structure
pnpm check:format:vue
cargo fmt --all -- --check
cargo metadata --no-deps --locked --format-version 1
```

未经用户明确要求，不主动构建、启动或运行项目测试，包括 `cargo check/test/clippy`、Vite、Tauri 和真实设备连接。Rust 工具链声明、依赖及解析结果见各 manifest、`Cargo.lock` 和 `pnpm-lock.yaml`；静态检查不等于编译或运行验收。已知依赖风险见 [公告记录](docs/01-product-requirements.md#10-依赖与公告核查)。

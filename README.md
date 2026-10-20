# Termeta

基于 Tauri 2、Rust、Vue 3 和 TypeScript 的桌面终端客户端，目标平台为 Windows、macOS、Linux。

源码包含 SSH、Telnet、本地 PTY、串口文本终端，以及加密主机管理、分屏广播、工作区存档、SFTP、转发、Snippets 和关闭时日志归档。支持官方账号与本地/离线模式：官方使用保险库主密钥自动同步业务数据，本地可用独立密码手动备份到 Gist/WebDAV。**当前版本尚未完成完整集成、三平台与发布验收。** 完成状态与限制以 [产品需求](docs/01-product-requirements.md) 为准。

## 开发入口

- [协作规则](AGENTS.md)：执行权限、代码与文档约束。
- [文档导航](docs/README.md)：架构、领域契约、依赖及恢复说明。
- [路线](docs/10-roadmap.md)：剩余工作与验收门槛。

Rust 领域核心位于 `crates/`，桌面装配与权限位于 `src-tauri/`，Vue 业务模块位于 `src/modules/`，唯一 IPC 门面位于 `src/services/desktop/`。

数据库采用 SQLCipher 加密格式 2、schema 7，可升级同格式 schema 2–6。物理备份必须配套保留 vault 元数据；官方同步与第三方备份均仅导出白名单业务快照。升级、改密和中断恢复见 [存储](docs/07-storage.md)，账号门禁、同步与合并边界见 [官方保险库同步](docs/26-official-vault-sync.md)。

## 静态检查

```sh
pnpm install --ignore-scripts
pnpm check:types
pnpm check:structure
pnpm check:i18n
pnpm check:format:vue
cargo fmt --all -- --check
cargo metadata --no-deps --locked --format-version 1
```

未经用户明确要求，不主动构建、启动或执行项目测试，包括 `cargo check/test/clippy`、Vite、Tauri 和真实主机/设备连接。静态检查不等于编译或运行验收；已知依赖风险见 [公告记录](docs/01-product-requirements.md#10-依赖与公告核查)。

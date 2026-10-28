# 依赖升级待处理项

更新：2026-10-28。本文维护不能直接应用的版本迁移、兼容阻碍与后续处理边界。当前版本以 manifest 和锁文件为准，选型、许可和持续风险见 [依赖说明](02-library-research.md)。候选版本是固定 release，不表示允许跳过兼容判断直接更新。

## 1. 需要单独迁移的依赖

| 依赖 | 当前基线 → 候选版本 | 暂缓原因 | 后续处理边界 |
| --- | --- | --- | --- |
| TypeScript | 6.0.3 → 7.0.2 | TypeScript 7 使用原生编译器；当前 vue-tsc 3.3.13 / Volar 2.4.28 仍通过 JavaScript 编译器源码注入 Vue 支持，不能直接使用新版二进制入口 | 保持 TypeScript 6；待 Vue 类型检查工具提供兼容路径后整体迁移，不能仅依据 peer dependency 范围替换 |
| libsqlite3-sys | 0.37.0 → 0.39.1 | 当前 SQLx SQLite 0.9.0 接受 `>=0.30.1,<0.38.0`，不能搭配 sys 0.39.1 | 等待 SQLx 支持新 sys 系列后共同选择 SQLCipher 和 OpenSSL 版本；保持 raw key、rekey、serialize/deserialize、外键及恢复约束，不改写已应用迁移 |
| GTK | 0.18.2 → 0.19.0 | Tauri 2.12 的 GTK 依赖仍为 0.18 系列，单独升级会造成类型及系统链接冲突；GTK 0.19.0 声明 MSRV 1.92 | 跟随 Tauri/Wry 的 GTK 兼容线处理 Linux 依赖；拖出适配复用同一 GTK 图，不能并行引入另一套系统绑定规避约束 |
| windows-core | 0.62.2 → 0.100.0 | 当前 `windows` 本体仍为 0.62.2，公开 COM 接口和实现宏使用配套 core 类型；新版 core 声明 MSRV 1.95 | 保持 windows/windows-core 配套；待 windows 本体提供对应版本后共同迁移文件拖出、COM 流和窗口适配 |

上述版本迁移不能仅靠刷新锁文件完成。数据库迁移及恢复约束见 [存储](07-storage.md)，OS 主密钥缓存见 [安全](08-security.md)，原生拖出边界见 [SFTP](16-sftp-workspace.md)。

固定版本依据：

- TypeScript：[7.0.2 release](https://github.com/microsoft/TypeScript/releases/tag/v7.0.2)、[vue-tsc 3.3.13 包](https://registry.npmjs.org/vue-tsc/-/vue-tsc-3.3.13.tgz)、[Volar 2.4.28 包](https://registry.npmjs.org/@volar/typescript/-/typescript-2.4.28.tgz)。
- 数据库：[SQLx SQLite 0.9.0](https://docs.rs/crate/sqlx-sqlite/0.9.0/source/Cargo.toml)、[libsqlite3-sys 0.37.0](https://docs.rs/crate/libsqlite3-sys/0.37.0/source/Cargo.toml)、[libsqlite3-sys 0.39.1](https://docs.rs/crate/libsqlite3-sys/0.39.1/source/Cargo.toml)。
- 平台绑定：[Tauri 2.12.5 manifest](https://docs.rs/crate/tauri/2.12.5/source/Cargo.toml)、[GTK 0.19.0](https://docs.rs/crate/gtk/0.19.0/source/Cargo.toml)、[windows 0.62.2](https://docs.rs/crate/windows/0.62.2/source/Cargo.toml)、[windows-core 0.100.0](https://docs.rs/crate/windows-core/0.100.0/source/Cargo.toml)。

## 2. 持续依赖风险

版本迁移与已知风险分别处理，不能把停止维护等同于可利用漏洞，也不能把其他库已更新视为下列风险消失。

- RSA 的 [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html) 仍涉及实际 SSH 签名依赖，尚无可直接应用的修复版本。
- Linux glib 0.18.5 的 [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) 仍受 GTK 兼容线约束；升级 Tauri 2.12 本身不代表该公告已解决。
- proc-macro-error、zvariant/zvariant_derive/zbus_names 的停止维护公告需要跟踪上游替代和传递依赖调整，不能用无关主版本的强制覆盖消除锁文件条目。当前风险和公告入口集中在 [依赖说明](02-library-research.md#4-已知依赖风险)。

后续升级同时维护 manifest、锁文件、许可与选型文档；MSRV 需要覆盖整个目标平台依赖图，不由单个直接依赖的声明推定。

# Termvexa

基于 Tauri 2、Rust、Vue 3 和 TypeScript 的模块化桌面 SSH 客户端，目标平台为 Windows、macOS、Linux。

首期源码已包含主机/分组管理、SSH 信任与认证、保留历史的多标签终端、SQLite 与系统凭据库、快捷键和独立终端配色。当前为**实现待验证**：未编译、启动、连接真实 SSH 或完成三平台验收，不能视为可发布版本。

## 目录

- `crates/terminal-core`：协议无关的会话模型、actor、取消与受界数据流。
- `crates/terminal-ssh`：russh 适配、主机密钥、认证、PTY/Shell。
- `crates/terminal-storage`：SQLite migration、主机/信任/设置/历史。
- `crates/terminal-security`：OS 凭据库、私钥授权读取。
- `src-tauri/src/commands`：命令 DTO、窗口所有权、Channel 与用例集成。
- `src/modules`：主机、终端、工作区、设置；`src/services/desktop` 为唯一 IPC 门面。

开发前阅读 [协作规则](AGENTS.md)、[文档导航](docs/README.md)、[首期实施状态](docs/17-implementation-status.md) 和 [路线](docs/10-roadmap.md)。Local、Serial、SFTP、隧道和分屏仍为后续阶段。

## 静态检查

```sh
pnpm install --ignore-scripts
pnpm exec vue-tsc --noEmit
pnpm exec prettier --check src package.json
cargo fmt --all -- --check
cargo metadata --no-deps --locked --format-version 1
```

Rust 工具链下限暂按已选 russh/core 的 1.89 声明，实际依赖组合与平台工具链待验证。根 `Cargo.lock` 和 `pnpm-lock.yaml` 固定解析结果；数据库位于 Tauri 应用数据目录，秘密只保存在用户明确选择的系统凭据服务。没有系统服务时不回退明文。

仓库禁止未经用户明确要求主动构建/运行，包括 cargo check/test/clippy、Vite、Tauri 与真实 SSH 验证。格式化和不生成项目产物的静态检查可以执行。依赖公告检查仍有未关闭条目，见实施状态；不应将源码交付或静态通过当作验收或安全审计通过。

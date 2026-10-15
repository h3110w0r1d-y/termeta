# Termvexa

基于 Tauri 2、Rust、Vue 3 和 TypeScript 的模块化桌面 SSH 客户端，目标平台为 Windows、macOS、Linux。

首期源码已包含主机/分组管理、SSH 信任与认证、保留历史的多标签终端、SQLite与系统凭据库、快捷键、独立终端配色、分屏广播，以及单SSH终端的SFTP浏览/上传/下载。当前为**实现待验证**：用户已有部分macOS终端使用反馈，当前版本尚未完成编译/运行与三平台验收，不能视为可发布版本。

## 目录

- `crates/terminal-core`：协议无关的会话模型、actor、取消与受界数据流。
- `crates/terminal-ssh`：russh 适配、主机密钥、认证、PTY/Shell。
- `crates/terminal-storage`：SQLite migration、主机/信任/设置/历史。
- `crates/terminal-security`：OS 凭据库、私钥授权读取。
- `crates/terminal-files` / `terminal-sftp`：文件端口、受界传输与复用SSH连接的SFTP适配。
- `src-tauri/src/commands`：命令 DTO、窗口所有权、Channel 与用例集成。
- `src/modules`：主机、终端、工作区、设置；`src/services/desktop` 为唯一 IPC 门面。

开发前阅读 [协作规则](AGENTS.md)、[文档导航](docs/README.md)、[首期实施状态](docs/17-implementation-status.md) 和 [路线](docs/10-roadmap.md)。Local、Serial、隧道、布局恢复与独立SFTP连接仍为后续范围。SFTP支持原生拖入当前目录、确认后直接覆盖普通文件和目录合并；macOS文件/目录承诺拖出已落源码，Windows/Linux拖出未实现。

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

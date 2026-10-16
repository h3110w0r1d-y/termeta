# Termvexa

基于 Tauri 2、Rust、Vue 3 和 TypeScript 的模块化桌面 SSH 客户端，目标平台为 Windows、macOS、Linux。

首期源码已包含主机/分组管理、SSH 信任与认证、保留历史的多标签终端、基础本地PTY终端、SQLCipher加密保险库与系统主密钥缓存、快捷键、独立终端配色、分屏广播，以及单SSH终端的SFTP浏览/上传/下载。当前为**实现待验证**：用户已有部分macOS终端使用反馈，当前版本尚未完成编译/运行与三平台验收，不能视为可发布版本。

## 目录

- `crates/terminal-core`：协议无关的会话模型、actor、取消与受界数据流。
- `crates/terminal-ssh`：russh 适配、主机密钥、认证、PTY/Shell。
- `crates/terminal-storage`：SQLCipher、schema、主机/信任/设置/历史。
- `crates/terminal-security`：Argon2id、保险库解锁与 OS 主密钥缓存。
- `crates/terminal-local`：系统Shell与本地PTY，复用统一会话/ACK管线；自定义启动配置和完整进程树清理待补齐。
- `crates/terminal-files` / `terminal-sftp`：文件端口、受界传输与复用SSH连接的SFTP适配。
- `src-tauri/src/commands`：命令 DTO、窗口所有权、Channel 与用例集成。
- `src/modules`：主机、终端、工作区、设置；`src/services/desktop` 为唯一 IPC 门面。

开发前阅读 [协作规则](AGENTS.md)、[文档导航](docs/README.md)、[需求与实现状态](docs/01-product-requirements.md#8-当前实现与限制) 和 [路线](docs/10-roadmap.md)。固定 SFTP 双栏已支持独立 SSH/SFTP 连接与栏间对拷；基础Serial文本终端已接入启动配置弹窗，详见[串口设计](docs/19-serial-terminal.md)；DTR/RTS控制、HEX、隧道、布局恢复、本地文件浏览与通用连接租约仍为后续范围。SFTP支持原生拖入当前目录、确认后直接覆盖普通文件和目录合并；macOS文件/目录承诺拖出已落源码，Windows/Linux拖出未实现。

## 静态检查

```sh
pnpm install --ignore-scripts
pnpm exec vue-tsc --noEmit
pnpm exec prettier --check src package.json
cargo fmt --all -- --check
cargo metadata --no-deps --locked --format-version 1
```

Rust 工具链下限暂按已选 russh/core 的 1.89 声明，实际依赖组合与平台工具链待验证。根 `Cargo.lock` 和 `pnpm-lock.yaml` 固定解析结果；数据库位于 Tauri 应用数据目录，业务数据与秘密统一存入 SQLCipher 加密数据库，配套 vault.json 须一同备份；系统凭据服务只缓存主密钥，不可用时通过保险库密码恢复，不回退明文。

仓库禁止未经用户明确要求主动构建/运行，包括 cargo check/test/clippy、Vite、Tauri 与真实 SSH 验证。格式化和不生成项目产物的静态检查可以执行。依赖公告检查仍有未关闭条目，见[依赖公告记录](docs/01-product-requirements.md#10-依赖与公告核查)；不应将源码交付或静态通过当作验收或安全审计通过。

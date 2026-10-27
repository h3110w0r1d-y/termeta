# Termeta

基于 Tauri 2、Rust、Vue 3 和 TypeScript 的桌面终端客户端，面向 Windows、macOS 和 Linux。将远程连接、本地终端、文件传输和端口转发放在同一个工作台中，连接配置与凭据保存在本机加密保险库。

当前版本为 0.1.0。功能完成范围见 [产品需求与状态](docs/01-product-requirements.md)，待完成事项见 [开发路线](docs/10-roadmap.md)。

## 功能

| 领域 | 当前能力 |
| --- | --- |
| 终端连接 | SSH、Telnet、本地 Shell 和串口文本终端 |
| SSH 认证 | 密码、公钥、加密私钥口令，以及服务器协商的交互认证；主机密钥核验与信任管理 |
| 连接管理 | 主机分组、标签、搜索、批量打开，以及独立凭据管理 |
| 工作台 | 多标签、分屏、焦点切换、显式输入广播；保存和打开工作区布局 |
| SFTP | 单终端侧栏、双远端浏览、上传下载、远端对拷、文件操作及原生拖放 |
| 端口转发 | 本地、远程和 SOCKS5 转发，独立连接并手动启停 |
| 日常工具 | 命令片段、终端搜索、关闭视图时归档会话日志、只读彩色日志回看 |
| 外观与语言 | 应用深色主题与自定义主色、独立终端配色、12 个内置字体家族、可配置快捷键；简体中文、繁体中文、英语、日语和韩语 |
| 备份与恢复 | 使用独立备份密码手动备份到 GitHub Gist 或 WebDAV，选择版本后新增合并恢复 |

SFTP 的完整传输管理、本地文件端点、SSH ProxyJump/Agent、自动备份和双向同步仍未完成；签名、公证、自动更新及安装体验也有待完善。目标平台不包含 Android、iOS 或浏览器终端服务。

## 数据与安全

- 主机、凭据、设置和会话日志保存在本机 SQLCipher 加密数据库中；保险库独立初始化与解锁。
- SSH 在服务器身份核验后才提交认证材料；未知密钥需要明确确认，已保存密钥变化时阻断连接。
- 默认无遥测和崩溃上传。远端 OSC 52 剪贴板读写被禁用，外链需要用户操作才能交给系统浏览器。
- 第三方备份使用独立密码加密，只导出白名单业务数据，包含保存的凭据和命令片段，不包含应用设置、会话日志或云端认证信息。
- 会话日志可能保留命令、登录回显和其他敏感输出；Telnet 使用明文传输。

请妥善保管保险库密码和备份密码。数据库物理备份需要同时保留配套 vault 元数据，恢复前先阅读 [存储与恢复](docs/07-storage.md)。安全边界、现有局限和已知依赖风险分别见 [安全说明](docs/08-security.md)、[备份说明](docs/22-data-backup.md) 和 [依赖风险](docs/02-library-research.md#4-已知依赖风险)。

## 本地开发

### 环境准备

- Node.js 22.18 或更高版本，以及支持当前锁文件的 pnpm。
- 支持 Rust edition 2024 及锁定依赖的稳定版 Rust 工具链，具体依赖要求见 [依赖说明](docs/02-library-research.md)。
- 对应平台的 [Tauri 2 系统依赖](https://v2.tauri.app/start/prerequisites/)：Windows 的 MSVC 工具链与 WebView2、macOS 的 Xcode Command Line Tools，或 Linux 的 WebKitGTK/GTK 及原生构建工具。

SQLCipher、OpenSSL 等依赖涉及原生编译，请按目标平台准备工具链。依赖版本以 [Cargo.lock](Cargo.lock) 和 [pnpm-lock.yaml](pnpm-lock.yaml) 为准。

### 安装与启动

在仓库根目录安装前端依赖，禁用安装生命周期脚本：

```sh
pnpm install --frozen-lockfile --ignore-scripts
```

启动桌面开发环境：

```sh
pnpm tauri dev
```

生成桌面安装包：

```sh
pnpm tauri:build
```

`pnpm dev` 仅启动前端开发服务器，`pnpm build` 仅检查并打包前端。桌面功能依赖 Tauri 宿主；开发和打包命令会编译或启动项目。

### 静态检查

```sh
pnpm check:types
pnpm check:structure
pnpm check:i18n
pnpm check:format:vue
cargo fmt --all -- --check
```

以上命令只检查源码、类型、格式和结构，不构建或启动应用。

## 代码结构

```text
src/              Vue 界面、业务模块与桌面 IPC 门面
src-tauri/        Tauri 桌面壳、权限和原生平台适配
crates/           会话核心、协议、存储、安全及文件服务
resources/        共享资源清单、品牌源图与资源说明
public/           随应用分发的字体、许可证和静态文件
scripts/          静态检查、资源生成与开发维护工具
docs/             产品需求、架构与领域文档
```

协议、网络、设备、凭据和数据库由 Rust 管理；前端通过 `src/services/desktop/` 调用桌面能力。架构与模块边界见 [系统架构](docs/03-architecture.md)。

## 参与开发

开始前请阅读 [协作指南](AGENTS.md) 和 [文档导航](docs/README.md)。提交问题时请提供应用版本、操作系统、复现步骤及预期结果，并移除凭据、真实主机信息和敏感终端输出。

修改功能时同步接口、五种语言文案和相关文档；已应用的数据库迁移必须保持原始字节，后续变化通过新增迁移完成。构建、启动和真实主机或设备连接遵循协作指南中的执行约束。

## 许可与第三方资源

Termeta 项目源码采用 [GNU General Public License v3.0](LICENSE)（`GPL-3.0-only`）许可。你可以依照许可证使用、修改和分发；分发项目或其修改版本时，须遵守 GPL-3.0 对许可证保留及相应源代码提供等要求。

字体及依赖保留各自的许可证与版权说明；品牌图稿、终端配色、拼音表和 SSH 系统图标的来源及许可边界见 [资源说明](resources/README.md) 和 [字体说明](resources/fonts/README.md)。部分资源的再分发授权尚待确认，公开源码不代表取得这些资源的额外授权。

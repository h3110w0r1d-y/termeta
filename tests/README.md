# 静态检查与运行验收边界

更新：2026-10-27。当前仓库不保留 Node 或 Rust 自动测试源码、测试辅助函数及共享测试样例；`tests/` 仅保留本文作为验证说明入口。

## 静态检查

| 命令 | 检查范围 |
| --- | --- |
| `pnpm check:types` | Vue 与 TypeScript 类型 |
| `pnpm check:structure` | 模块导入、表单、IPC 命令和权限入口等结构约束 |
| `pnpm check:i18n` | 五语言语义 key、命名参数和文案引用 |
| `pnpm check:format:vue` | Vue 文件格式 |
| `cargo fmt --all -- --check` | Rust 文件格式 |

这些命令不构建或启动应用。静态检查脚本保留在 [scripts](../scripts/)，不能证明运行时行为、SQLCipher 迁移、权限或真实协议兼容性。

## 运行验收

编译、启动和真实主机或设备连接遵循 [协作规则](../AGENTS.md)，须用户明确授权。Rust 的 `check`、`test`、`clippy` 也会触发编译，不属于上述静态检查。

当前运行门槛只维护在 [产品需求](../docs/01-product-requirements.md#11-运行验收边界)，领域文档维护独特行为。运行证据记录日期、代码版本、平台/架构、工具链、服务端或设备、步骤及实际结果，不含凭据或敏感输出。

历史 macOS core/Serial 定向编译和测试、指定 ESP32-C3 输出/关闭及 xterm 无界面缓冲比较仅限对应版本与场景，见 [串口记录](../docs/19-serial-terminal.md#5-已有验证与诊断入口)。历史证据与静态检查通过均不代表当前完整应用或三平台验收通过。

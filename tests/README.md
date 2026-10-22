# 测试入口与验证边界

Node 用例要求 Node 22.18+；纯逻辑用例直接导入 TypeScript，保险库门禁用例使用内存 Vue 宿主与模拟 IPC。Rust 测试会编译。遵循 [协作规则](../AGENTS.md)，未经用户明确授权不执行项目测试、构建/启动应用或连接主机/设备。

## Node 用例

| 文件 | 覆盖领域 |
| --- | --- |
| [tab-insertion.test.mjs](tab-insertion.test.mjs) | 标签命中、排序与坐标换算 |
| [workbench-layout.test.mjs](workbench-layout.test.mjs) | 分屏、迁移、收缩、比例与不变性 |
| [terminal-broadcast.test.mjs](terminal-broadcast.test.mjs) | 用户输入/自动应答隔离、滚轮编码与广播边界 |
| [terminal-activity.test.mjs](terminal-activity.test.mjs) | 关闭提醒和连接代次隔离 |
| [terminal-working-directory.test.mjs](terminal-working-directory.test.mjs) | 目录上报/标题解析、路径边界、主目录展开与上报优先级 |
| [sftp-directory-follow.test.mjs](sftp-directory-follow.test.mjs) | 目录跟随启停、忙碌/隐藏时合并、断线与卸载清理 |
| [sftp-file-drag.test.mjs](sftp-file-drag.test.mjs) | 内部路径落点/外部下载互斥、取消/代次、路径转义及 POSIX Shell 参数解析 |
| [terminal-appearance.test.mjs](terminal-appearance.test.mjs) | 终端表面颜色混合、透明通道及 Naive UI 表格颜色解析 |
| [host-search.test.mjs](host-search.test.mjs) | 拼音/字段搜索 |
| [file-entries.test.mjs](file-entries.test.mjs) | 排序、过滤、分页与链接展示 |
| [state-races.test.mjs](state-races.test.mjs) | 主机旧读与成功写入（含批量排序/迁组）交错、设置订阅单飞/失败重试/卸载、隐藏文件即时过滤与保存失败回退 |
| [session-control.test.mjs](session-control.test.mjs) | 控制信封、早于响应的事件、旧流与迟到挑战隔离 |
| [terminal-lifecycle.test.mjs](terminal-lifecycle.test.mjs) | 统一关闭、重复调用、归档失败重试和共享并发预算 |
| [terminal-keyboard.test.mjs](terminal-keyboard.test.mjs) | 终态回车重连、连接状态白名单、焦点/IME/长按/弹层/快捷键边界及回车消费 |
| [terminal-mouse.test.mjs](terminal-mouse.test.mjs) | 右键/中键独立动作、粘贴单次消费、远端鼠标协议互斥、原生菜单/选区粘贴抑制和监听清理 |
| [terminal-stream.test.mjs](terminal-stream.test.mjs) | 终态先于尾帧时等待解析、缺帧超时与重试 |
| [vault-gate.test.mjs](vault-gate.test.mjs) | 本机保险库初始化、自动解锁、密码解锁与失败重试 |

授权后的入口：

```sh
node --test tests/*.test.mjs
```

## Rust 回归源码

用例就近维护在各 crate 和壳模块中，具体断言以源码为准：core 覆盖会话/流控/取消；协议 crate 覆盖参数、编码、协商及转发；storage 覆盖持久化/事务/迁移/备份新增合并，以及批量主机排序/迁组的中断回滚、版本与范围冲突、凭据保留及重开顺序；security 覆盖主密钥/包认证/重加密恢复；壳覆盖 DTO、权限和设置契约，以及预览代次/独立恢复提交。Local 新增有界输入桥背压，SSH 新增端点身份，core 新增附属资源幂等撤销与锁外回调，storage 新增公共凭据原子失败/历史 pending 清理回归源码；本轮均未编译或执行。

## 证据与静态检查

历史授权调试仅有 macOS core/Serial 的定向编译及 15 项测试、指定 ESP32-C3 输出/关闭和 xterm 无界面缓冲比较等局部证据，见 [串口记录](../docs/19-serial-terminal.md#5-已有验证与诊断入口)。其他新增回归源码尚未完成编译/运行验收，测试存在不等于通过。

`pnpm check:types`、`pnpm check:structure`、`pnpm check:i18n`、`pnpm check:format:vue` 和 `cargo fmt --all -- --check` 只做静态检查，不构建/启动项目。静态解析、内存 SQLite 迁移模型或历史检查不能证明当前 SQLCipher/桌面运行通过。

当前运行门槛只维护在 [产品需求](../docs/01-product-requirements.md#11-运行验收边界)，领域文档维护独特行为。运行证据记录日期、代码版本、平台/架构、工具链、服务端/设备、步骤及结果，不含凭据或敏感输出。

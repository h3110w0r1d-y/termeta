# 快捷键设置与输入路由

更新：2026-10-18。应用内快捷键支持分类编辑、录制、多绑定、禁用、恢复默认与 JSON 导入导出。当前没有系统后台热键、多段 chord、按平台独立覆盖或原生菜单 accelerator 的通用同步。

## 1. 默认动作

`Primary` 在 macOS 对应 Command，在 Windows/Linux 对应 Control。按钮、菜单和键盘共用 [命令注册表](../src/services/commands/registry.ts)。

| 动作 | macOS | Windows/Linux |
| --- | --- | --- |
| 新建标签页 | Cmd+T | Ctrl+T |
| 新连接入口 | Cmd+N | Ctrl+Shift+N |
| 关闭标签 | Cmd+W | Ctrl+Shift+W |
| 下一个/上一个标签 | Cmd+Shift+] / [ | Ctrl+PageDown / PageUp |
| 打开设置 | Cmd+,（原生菜单） | Ctrl+, |
| 新建本地终端 | Cmd+L | Ctrl+Shift+L |
| 复制/粘贴 | Cmd+C / V | Ctrl+Shift+C / V |
| 搜索终端 | Cmd+F | Ctrl+Shift+F |
| 打开终端链接 | Cmd+主键单击 | Ctrl+主键单击 |
| 滚动到底部 | 未绑定 | 未绑定 |

新建标签页和新连接入口都打开全局连接选择弹窗，快速连接由弹窗进入。SFTP 没有独立快捷键。Windows/Linux 默认 Ctrl+C 发送远端中断，Ctrl+L 交给 Shell。

## 2. 设置与路由边界

仅接受 `keymap.schemaVersion=2`。应用动作使用 `workbench.*` 或 `settings.open`，作用域为 application、上下文为 workbench；终端动作使用 `terminal.*`、terminal 作用域与上下文。前后端校验注册动作、触发器、修饰键、上下文及冲突，不接受脚本、Shell 命令或宏。旧版本、旧命名和未知动作拒绝导入，不静默转默认值。

读取和导入保留用户绑定集合，不补充缺失动作；新安装或主动恢复默认才获取完整默认集。录制只捕获组合，不执行动作或向远端发送输入。

应用动作在前台跨页面生效，包括普通输入框；终端动作仅终端内生效。路由优先处理录制、IME、模态和弹层，再匹配终端动作、应用动作，剩余输入交给 xterm。主机搜索下拉允许应用动作，其他弹层优先。仅实际消费动作时阻止默认行为；用户显式将应用动作绑定到编辑按键时，该绑定优先。

标准终端粘贴匹配已启用绑定后放行系统 paste，由宿主统一执行 64 KiB 限制、多行确认、括号粘贴和广播代次校验；自定义绑定使用显式剪贴板读取，可能受 WebView 授权限制。粘贴不自动补 Enter。macOS 无其他修饰键时，Option+左右方向键可转换为 ESC b/f，自定义绑定优先。

## 3. macOS 原生菜单与关闭

固定 Cmd+, 由 AppKit 应用内事件监视器交给原生设置菜单，成功处理后消费事件，避免逗号进入输入框或重复触发。前端不默认绑定该组合，残留同组合绑定也不执行；用户仍可配置其他设置快捷键。此组合不受前端模态或录制拦截，但打开设置仍须通过解锁与窗口权限检查，不属于系统后台热键。

模态中 Cmd+W 被消费，不关闭标签或主窗口，保护独立于关闭标签的用户绑定。原生文件/窗口菜单的关闭项没有 Cmd+W，显式菜单及窗口按钮继续走关闭确认。设置窗口不注册终端或工作台命令，Cmd/Ctrl+W 等待设置写入后关闭，录制和模态优先。

原生入口见 [macos_menu](../src-tauri/src/macos_menu.rs) 和 [settings_window](../src-tauri/src/settings_window.rs)。原生行为变化需要桌面壳编译生效，前端热更新不能验证。

## 4. 终端链接与验证

自动 URL 与 OSC 8 链接均须匹配配置的修饰键主键点击；普通点击/拖动保留选择及远端鼠标语义。Rust 再校验 http/https、长度与控制字符，拒绝 file/javascript 等协议；输出不得自动打开链接。

配置校验位于 [settings.rs](../src-tauri/src/settings.rs)，终端粘贴与链接执行位于 [registry.ts](../src/modules/terminal/registry.ts)。IME、AltGr、原生菜单、焦点、选区及一次动作只执行一次仍需运行验收，范围统一见 [产品需求](01-product-requirements.md)。

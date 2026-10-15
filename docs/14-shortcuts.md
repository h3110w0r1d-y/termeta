# 全局快捷键与终端快捷键设置

日期：2026-10-15。新增需求基线，尚未实现。M2 提供快捷键注册表、设置与终端动作；依赖 Local/SFTP 的动作随对应模块启用，不改变 SSH 优先路线。

## 1. 分类与作用域

设置界面一级分为“全局快捷键”和“终端快捷键”，所有产品动作尽量可自定义，明确展示动作所属模块与当前是否可用。

| 分类 | scope | 生效范围 | 示例 |
| --- | --- | --- | --- |
| 全局快捷键 | application | 应用窗口处于前台，跨终端/主机/SFTP/设置页面 | Cmd+L 打开新本地终端、切换 Tab、打开设置 |
| 终端快捷键 | terminal | 当前终端宿主获得焦点或指针命中终端链接 | Cmd/Ctrl+点击链接打开浏览器、复制/搜索/粘贴 |
| 全局快捷键的可选后台模式 | system | 应用不在前台也可触发；独立 opt-in | 后续显示主窗口/快速新建终端 |

本项目所称“全局”默认指应用内全局；系统后台热键在设置中明确标“系统级”，不能把所有应用快捷键自动注册到 OS。应用内 keyboard router 不依赖系统全局插件；后续系统模式需验证平台支持和注册结果。

## 2. 初始动作与平台默认值

`Primary` 在 macOS 映射 Meta/Command，在 Windows/Linux 映射 Control；界面显示实际平台键名。用户可直接设具体修饰键，不强制使用 Primary。

| commandId | 分类 | macOS 初始值 | Windows/Linux 初始值 | 条件 |
| --- | --- | --- | --- | --- |
| `workspace.newLocalTerminal` | 全局 | Cmd+L | Ctrl+Shift+L | Local 工厂已注册；未实现时禁用并显示原因 |
| `workspace.newConnection` | 全局 | Cmd+N | Ctrl+Shift+N | SSH 工作台可用 |
| `workspace.newSftpTab` | 全局 | Cmd+Shift+S | Ctrl+Alt+S | SFTP 模块可用；冲突可修改 |
| `workspace.closeTab` | 全局 | Cmd+W | Ctrl+Shift+W | 活跃终端/传输遵循关闭确认 |
| `workspace.nextTab` / `previousTab` | 全局 | Cmd+Shift+] / [ | Ctrl+PageDown / PageUp | 应用内，不自动占用系统同组合 |
| `settings.open` | 全局 | Cmd+, | Ctrl+, | 正常编辑状态可用 |
| `terminal.copy` / `paste` | 终端 | Cmd+C / Cmd+V | Ctrl+Shift+C / Ctrl+Shift+V | 离线仍可复制；粘贴仅连接成功后写远端 |
| `terminal.find` | 终端 | Cmd+F | Ctrl+Shift+F | 离线也可搜索 |
| `terminal.openLink` | 终端 | Cmd+鼠标主键单击 | Ctrl+鼠标主键单击 | 只允许受控 http/https，终端断线后也可用 |
| `terminal.toggleSftpPanel` | 终端 | Cmd+Shift+E | Ctrl+Shift+E | 当前为可用 SSH 终端；位置按设置 |
| `terminal.scrollToBottom` | 终端 | 可自定义，初始无绑定 | 可自定义，初始无绑定 | 不清空历史 |

Windows/Linux 不默认使用 Ctrl+L 打开本地终端，避免抢占 Shell 常用清屏键；用户可显式改成 Ctrl+L，设置界面提示其会截获远端输入。macOS 的 Cmd+L 按用户要求默认创建新本地终端 Tab，而不是在 SSH 远端执行命令。未实现 Local 时保留动作配置和禁用说明，不启动系统 shell 旁路，不假称已具备 Local 功能。

用户可为同一动作设置多个组合、移除绑定、单项/分类/全部恢复默认、导入/导出配置。M2 至少支持单次按键组合和修饰键+鼠标点击；多段 chord 作为扩展字段，未实现时导入需提示，不能误执行前半段。

## 3. 设置界面

列表显示动作名称、分类/上下文、当前绑定、默认绑定、平台、启用状态、冲突与说明，支持按名称/按键搜索。录制模式只记录组合，不执行产品动作、不向远端发送、不注册 OS 热键；支持取消与清除。鼠标组合编辑器单独选择修饰键、主键单击，不用只能录键盘的输入框冒充支持。

冲突分为相同上下文直接冲突、不同上下文可共存、OS/菜单保留组合、远端常用控制键冲突；保存前给出具体动作与范围，不静默覆盖或随机选择一个。优先阻止同 scope/context 的完全重叠规则；允许用户明确替换或改变条件。导入先预览差异与冲突，版本不支持的条目不自动激活。

设置支持立即应用但持久化是事务式：保存失败恢复旧配置并提示。系统级热键变更先尝试注册新项、确认成功再撤销旧项；失败保持旧有效绑定，不能 UI 显示已生效但 OS 未注册。

## 4. 命令注册表与路由

统一 `CommandRegistry` 定义 commandId、labelKey、allowedScopes、availability、defaultBindings、可重复触发策略与 handler。按钮、菜单和键盘复用同一 command；可用性取决于实际工厂/文件源能力，不按 UI 是否渲染某按钮猜测。

`KeymapService` 解析版本化配置并解析平台键名；`ShortcutRouter` 对事件一次决策，成功消费才 preventDefault/stopPropagation。xterm 通过受控自定义按键处理入口对接该 router，不同时重复绑定全局 keydown 和本地 handler 执行同一 command。原生菜单 accelerator 也必须去重；不使用 keyup 再执行一次。

优先级：快捷键录制/IME composition → 原生或模态对话框 → 当前输入控件/菜单/焦点上下文 → 终端专用绑定 → 应用全局绑定 → xterm 默认编码发送远端。上下文动作优先并不允许静默遮蔽配置；冲突检查解释当前设置可能不生效的场景。应用全局 Cmd+L 在常规页面与终端中都可用，在认证/保存确认等模态流程中按动作策略暂缓，不能绕过未完成认证。

文本编辑保留必要的系统复制/选区行为，IME composition 不截获中间按键，AltGr 和键盘布局不能简单当成 Ctrl+Alt。默认用规范化逻辑按键，展示物理键位/布局差异；未来物理 key code 模式另设选项，不混写一套配置。

Ctrl+C 在 Windows/Linux 终端默认继续作为远端中断；若用户主动配置冲突，提示被截获的行为。危险的终端清屏/清空滚屏是独立显式动作，断线/重连流程不得调用它。

## 5. 修饰键点击链接

终端链接包括自动识别 URL 与可支持的显式终端超链接，两条路径共用 `LinkActivationPolicy`。默认只在配置的 Cmd/Ctrl+主键单击且未拖选时打开；普通单击/拖动保持选择与远端应用鼠标语义。满足打开条件时避免再向远端发送同一鼠标动作；修饰键与选择/鼠标报告的协调需实测。

URI 交给 Rust 受控 opener 再校验 http/https、长度与控制字符；拒绝 file/javascript/命令 scheme，用户自定义按键不能扩大 scheme 白名单。链接显示与真实目标不同时悬浮提示真实目标，禁止远端自动触发打开。状态分界中的本地诊断也不允许注入未校验 URL。

## 6. 配置模型草案

```ts
interface ShortcutBindingV1 {
  id: string;
  commandId: string;
  scope: "application" | "terminal" | "system";
  context: "workspace" | "terminal" | "sshTerminal" | "fileBrowser";
  trigger:
    | { type: "key"; key: string; modifiers: string[] }
    | { type: "pointer"; button: "primary"; event: "click"; modifiers: string[] };
  platform: "all" | "macOS" | "windows" | "linux";
  enabled: boolean;
}
```

实际键值/modifier 用白名单枚举，不能允许任意字符串变成 JS 回调或 OS 命令。导入只映射已注册 commandId，不接受脚本、shell、远端输入宏；snippet 的执行属于另一套显式授权功能。存储采用 settings 中版本化 `keymap.bindings`，默认值在注册表维护，持久化用户覆盖；新版本动作合并不覆盖已有设置。

## 7. 原生能力与验收

官方 Global Shortcut 插件可作为系统级模式候选，具体组合/桌面环境注册结果需真实验证；本轮未安装，也不宣称 OS 保留键都可重映射。[Tauri 官方文档](https://v2.tauri.app/plugin/global-shortcut/)

验收包括全局/终端分类编辑、Cmd+L 可用性、Cmd/Ctrl+点击 URL、重复触发去重、焦点上下文、导入冲突、禁用恢复、IME/AltGr、原生菜单一致性、系统模式注册失败回滚。系统级后台模式作为 M5/P1 扩展；M2/P0 必须交付应用内全局和终端快捷键设置，不能仅交付硬编码默认值。

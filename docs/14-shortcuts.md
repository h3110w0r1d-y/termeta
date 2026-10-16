# 快捷键设置与路由

应用内全局与终端快捷键已实现注册表、分类编辑、录制、多绑定、禁用、恢复默认及JSON导入导出。IME、平台键位、焦点与鼠标去重仍待运行验收；系统后台热键、原生菜单accelerator和多段chord未实现。

## 当前动作

`Primary` 在macOS对应Command，在Windows/Linux对应Control。注册表位于 `src/services/commands/registry.ts`，设置模型为 `keymap.schemaVersion=1` 和 `bindings`；按钮/菜单与键盘复用动作处理入口。

| 动作              | macOS默认         | Windows/Linux默认        |
| ----------------- | ----------------- | ------------------------ |
| 新建标签页        | Cmd+T             | Ctrl+T                   |
| 新建SSH连接       | Cmd+N             | Ctrl+Shift+N             |
| 关闭标签          | Cmd+W             | Ctrl+Shift+W             |
| 下一个/上一个标签 | Cmd+Shift+] / [   | Ctrl+PageDown / PageUp   |
| 打开设置          | Cmd+,             | Ctrl+,                   |
| 复制/粘贴         | Cmd+C / V         | Ctrl+Shift+C / V         |
| 搜索终端          | Cmd+F             | Ctrl+Shift+F             |
| 打开终端链接      | Cmd+主键单击      | Ctrl+主键单击            |
| 滚动到底部        | 未绑定            | 未绑定                   |
| 新建本地终端      | Cmd+L | Ctrl+Shift+L |

“全局”指应用窗口前台的application作用域，终端动作属于terminal作用域。Local工厂已实现，workspace.newLocalTerminal 注册到本地PTY，macOS为Cmd+L，Windows/Linux为Ctrl+Shift+L。SFTP目前使用状态栏开关，没有注册独立SFTP标签或面板快捷键。Windows/Linux默认Ctrl+C仍发送远端中断，Ctrl+L仍交给Shell；用户可显式改变绑定。

## 设置与安全边界

设置草稿支持按名称/绑定搜索、录制键盘组合、编辑链接鼠标修饰键和JSON导入导出。录制只捕获组合，不执行动作或发送远端输入。保存时校验已注册commandId、scope/context、修饰键与按键白名单，拒绝同作用域重叠绑定；不接受脚本、Shell命令或输入宏。当前平台字段仅支持all，不承诺跨平台独立覆盖或OS保留键检测。

路由顺序为录制/IME → 模态及菜单上下文 → 终端动作 → 应用动作 → xterm默认编码。仅实际消费的动作阻止默认行为；普通页面输入框允许已配置的应用快捷键，终端专属动作仅在终端内生效，因此默认复制/粘贴与方向键仍交由输入控件。用户显式将应用动作绑定到编辑按键时，以应用绑定为准。模态弹窗阻止全部应用快捷键，录制控件及其他弹层仍优先。全局路由使用捕获阶段，但带data-shortcut-recorder的录制控件仍优先交由录制逻辑处理。配置随设置保存，不注册OS后台热键。

macOS仅在无其他修饰键、无IME/AltGraph/活动弹层时，将Option+左右方向键转换为ESC b / ESC f；自定义快捷键优先。不开启全局macOptionIsMeta，远端行编辑器决定最终跳词行为。

## 终端链接

自动URL与OSC 8链接共用修饰键策略：普通点击/拖动保持选区和远端鼠标语义，符合配置的主键点击才请求打开。Rust再次校验http/https、长度与控制字符，拒绝file/javascript等协议；远端输出不得自动打开链接。

xterm在mouseup激活链接，处理程序仅preventDefault，不stopPropagation，保证document选区监听先完成清理；下一轮任务调用受控opener，已释放终端不再执行延迟打开。桌面实测需覆盖选区、鼠标上报、IME/AltGr、弹层优先级和一次动作只执行一次，见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。

系统后台模式仅为后续范围，候选接口见 [Tauri Global Shortcut](https://v2.tauri.app/plugin/global-shortcut/)，当前未安装或注册。

`workspace.newTab`默认Primary+T，标签栏＋共用此命令打开全局新建标签页弹窗。原`workspace.newConnection`及Cmd+N/Ctrl+Shift+N同样打开该弹窗；快速连接通过弹窗中的按钮进入。读取旧设置或导入旧keymap时补充newTab默认绑定，已有该动作（包括禁用）不改写，按键冲突时新绑定禁用，绑定已达100条不追加；恢复默认可获取完整默认动作集。Rust设置白名单同步允许workspace.newTab，无新增IPC。

主页搜索结果下拉以data-application-shortcuts标记允许应用快捷键；全局路由仅对不含该标记的data-ui-popup-open弹层拦截。搜索框空查询、输入关键词及结果下拉打开时，均允许新建本地终端、新建标签页、切换标签及打开设置等应用动作；其他浮层和模态仍优先。

## macOS 弹窗关闭保护（2026-10-16）

弹窗存在data-modal-open时，路由在录制/IME等提前返回之前消费Cmd+W（preventDefault/stopPropagation），不关闭弹窗、标签或窗口；保护独立于关闭标签的用户绑定及启用状态，其他应用动作继续被模态拦截，编辑快捷键和Escape保持原行为。没有弹窗时仍按用户绑定关闭当前标签，不回退为关闭应用窗口。

Tauri 2.11.5会默认创建macOS菜单，其中预设Close Window自带Cmd+W，可绕过DOM处理。src-tauri/src/macos_menu.rs沿用默认菜单，只重建Window子菜单，以无accelerator的普通Close Window项替代预设关闭项，保留最小化/最大化、其他默认菜单和原生编辑快捷键。显式菜单关闭仍调用window.close并经过现有CloseRequested确认；不增加前后端模态状态同步或新IPC。此调整需要重新编译桌面壳，单独前端热更新不能移除原生快捷键。尚未编译运行验证。

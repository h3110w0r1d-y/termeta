# 快捷键设置与路由

应用内全局与终端快捷键已实现注册表、分类编辑、录制、多绑定、禁用、恢复默认及JSON导入导出。IME、平台键位、焦点与鼠标去重仍待运行验收；系统后台热键、应用快捷键与原生菜单 accelerator 的通用同步、多段 chord 未实现；macOS 设置菜单单独提供固定 Cmd+,。

## 当前动作

`Primary` 在macOS对应Command，在Windows/Linux对应Control。注册表位于 `src/services/commands/registry.ts`，设置模型为 `keymap.schemaVersion=2` 和 `bindings`；按钮/菜单与键盘复用动作处理入口。

| 动作              | macOS默认       | Windows/Linux默认      |
| ----------------- | --------------- | ---------------------- |
| 新建标签页        | Cmd+T           | Ctrl+T                 |
| 新连接入口        | Cmd+N           | Ctrl+Shift+N           |
| 关闭标签          | Cmd+W           | Ctrl+Shift+W           |
| 下一个/上一个标签 | Cmd+Shift+] / [ | Ctrl+PageDown / PageUp |
| 打开设置          | Cmd+,（原生菜单） | Ctrl+,                 |
| 复制/粘贴         | Cmd+C / V       | Ctrl+Shift+C / V       |
| 搜索终端          | Cmd+F           | Ctrl+Shift+F           |
| 打开终端链接      | Cmd+主键单击    | Ctrl+主键单击          |
| 滚动到底部        | 未绑定          | 未绑定                 |
| 新建本地终端      | Cmd+L           | Ctrl+Shift+L           |

“全局”指应用窗口前台的application作用域，终端动作属于terminal作用域。Local工厂已实现，workbench.newLocalTerminal 注册到本地PTY，macOS为Cmd+L，Windows/Linux为Ctrl+Shift+L。SFTP目前使用状态栏开关，没有注册独立SFTP标签或面板快捷键。Windows/Linux默认Ctrl+C仍发送远端中断，Ctrl+L仍交给Shell；用户可显式改变绑定。

## 设置与安全边界

快捷键仅接受版本 2，使用 `workbench.*` 命令 ID 和 `context: "workbench"`；终端动作仍使用 `terminal.*` / `terminal`，`settings.open` 的应用上下文同样为 `workbench`。读取、保存和 JSON 导入直接校验当前格式；旧版本、旧命名、未知动作和错误上下文均拒绝，不进行兼容转换或回退为默认绑定。JSON 导入继续检查绑定冲突。

读取和导入保留已配置的绑定集合、身份、触发器及启用状态，不自动补充缺失动作；新安装和用户主动恢复默认时才使用完整默认绑定。保存继续在设置专用队列中合并最新记录并完整校验，跨窗口通知后串行重读。外层 app 设置版本和数据库 schema 不变。当前版本及旧格式拒绝用例位于 `src-tauri/src/settings.rs`，尚未执行。

设置草稿支持按名称/绑定搜索、录制键盘组合、编辑链接鼠标修饰键和JSON导入导出。录制只捕获组合，不执行动作或发送远端输入。保存时校验已注册commandId、scope/context、修饰键与按键白名单，拒绝同作用域重叠绑定；不接受脚本、Shell命令或输入宏。当前平台字段仅支持all，不承诺跨平台独立覆盖或OS保留键检测。

路由顺序为录制/IME → 模态及菜单上下文 → 终端动作 → 应用动作 → xterm默认编码。仅实际消费的动作阻止默认行为；普通页面输入框允许已配置的应用快捷键，终端专属动作仅在终端内生效，因此默认复制/粘贴与方向键仍交由输入控件。用户显式将应用动作绑定到编辑按键时，以应用绑定为准。模态弹窗阻止全部应用快捷键，录制控件及其他弹层仍优先。全局路由使用捕获阶段，但带data-shortcut-recorder的录制控件仍优先交由录制逻辑处理。配置随设置保存，不注册OS后台热键。

macOS仅在无其他修饰键、无IME/AltGraph/活动弹层时，将Option+左右方向键转换为ESC b / ESC f；自定义快捷键优先。不开启全局macOptionIsMeta，远端行编辑器决定最终跳词行为。

## 终端链接

自动URL与OSC 8链接共用修饰键策略：普通点击/拖动保持选区和远端鼠标语义，符合配置的主键点击才请求打开。Rust再次校验http/https、长度与控制字符，拒绝file/javascript等协议；远端输出不得自动打开链接。

xterm在mouseup激活链接，处理程序仅preventDefault，不stopPropagation，保证document选区监听先完成清理；下一轮任务调用受控opener，已释放终端不再执行延迟打开。桌面实测需覆盖选区、鼠标上报、IME/AltGr、弹层优先级和一次动作只执行一次，见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。

系统后台模式仅为后续范围，候选接口见 [Tauri Global Shortcut](https://v2.tauri.app/plugin/global-shortcut/)，当前未安装或注册。

`workbench.newTab`默认Primary+T，标签栏＋共用此命令打开全局新建标签页弹窗。原`workbench.newConnection`及Cmd+N/Ctrl+Shift+N同样打开该弹窗；快速连接通过弹窗中的按钮进入。读取和导入不自动补充newTab绑定；恢复默认可获取完整默认动作集。Rust设置白名单同步允许workbench.newTab，无新增IPC。

主页搜索结果下拉以data-application-shortcuts标记允许应用快捷键；全局路由仅对不含该标记的data-ui-popup-open弹层拦截。搜索框空查询、输入关键词及结果下拉打开时，均允许新建本地终端、新建标签页、切换标签及打开设置等应用动作；其他浮层和模态仍优先。

## macOS 弹窗关闭保护

弹窗存在data-modal-open时，路由在录制/IME等提前返回之前消费Cmd+W（preventDefault/stopPropagation），不关闭弹窗、标签或窗口；保护独立于关闭标签的用户绑定及启用状态，其他应用动作继续被模态拦截，编辑快捷键和Escape保持原行为。没有弹窗时仍按用户绑定关闭当前标签，不回退为关闭应用窗口。

Tauri 2.11.5会默认创建macOS菜单，其中预设Close Window自带Cmd+W，可绕过DOM处理。src-tauri/src/macos_menu.rs沿用默认菜单，只重建Window子菜单，以无accelerator的普通Close Window项替代预设关闭项，保留最小化/最大化、其他默认菜单和原生编辑快捷键。显式菜单关闭仍调用window.close并经过现有CloseRequested确认；不增加前后端模态状态同步或新IPC。此调整需要重新编译桌面壳，单独前端热更新不能移除原生快捷键。尚未编译运行验证。

独立设置窗口：macOS 的固定 Cmd+, 仅由原生“设置”菜单处理并显示 ⌘,，菜单标题不带省略号。AppKit 应用内 KeyDown 监视器在事件进入 WebKit 前匹配精确 Command+,，调用现有主菜单 performKeyEquivalent；菜单处理成功后返回空事件，防止逗号进入输入框及同一按键重复触发。其他组合、后台应用或没有活动窗口时原样放行，不注册系统后台热键或申请辅助功能权限。监视器仅在主线程安装/持有，主窗口销毁时移除。前端 settings.open 默认未绑定；路由对残余 Cmd+, 直接放行，不执行已有同组合绑定，也不调用 preventDefault/stopPropagation。已有配置保留原样，无需重置；快捷键页说明该组合固定由系统菜单处理，用户仍可为 settings.open 录制其他组合，由主窗口调用 open_settings_window。Windows/Linux 保留前端默认 Ctrl+,。菜单事件复用 settings_window::open，保持解锁检查与单实例聚焦；原生组合不受前端模态/录制/弹层路由约束。设置窗口不注册终端/工作区命令，Cmd/Ctrl+W 仅等待设置写入后关闭当前窗口，模态框/快捷键录制优先。原生 Close Window 菜单按当前聚焦窗口执行，各窗口自行处理 CloseRequested。

用户反馈仅依赖菜单 accelerator、前端直接放行时，输入框激活的 Cmd+, 无效，而前端处理的 Cmd+N/Cmd+T 正常。因此 DOM 放行不能作为原生菜单收到事件的依据；本地监视器的编译和运行验证尚未执行，需要重新编译桌面壳后验证，前端热更新不足以启用。验收分别覆盖普通输入框、主机搜索（空查询/结果下拉）、终端及设置窗口，首次创建/已有窗口聚焦、中英输入法、锁定保险库、额外修饰键及一次按键只执行一次。排查时对照监视器是否匹配、performKeyEquivalent 返回值、菜单回调，以及捕获/冒泡阶段 keydown 的 key/code/metaKey/defaultPrevented/isComposing 和 beforeinput 的 inputType；不记录输入正文或其他按键。原生菜单事件不携带可取消的 DOM 输入事件，不能在打开设置后补调 preventDefault 修复已发生的输入。

标准终端粘贴快捷键（macOS Cmd+V、Windows/Linux Ctrl+Shift+V）匹配已启用的 terminal.paste 后放行系统默认行为，不调用 navigator.clipboard.readText()。系统 paste 事件由终端宿主捕获，继续执行 64 KiB 限制、多行确认、括号粘贴及广播代次校验；自定义粘贴绑定仍使用显式剪贴板读取，可能受 WebView 授权限制。待运行验证直接粘贴、多行确认/取消、仅输入一次、改绑/禁用与广播。

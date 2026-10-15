# 快捷键设置与路由

应用内全局与终端快捷键已实现注册表、分类编辑、录制、多绑定、禁用、恢复默认及JSON导入导出。IME、平台键位、焦点与鼠标去重仍待运行验收；系统后台热键、原生菜单accelerator和多段chord未实现。

## 当前动作

`Primary` 在macOS对应Command，在Windows/Linux对应Control。注册表位于 `src/services/commands/registry.ts`，设置模型为 `keymap.schemaVersion=1` 和 `bindings`；按钮/菜单与键盘复用动作处理入口。

| 动作              | macOS默认         | Windows/Linux默认        |
| ----------------- | ----------------- | ------------------------ |
| 新建SSH连接       | Cmd+N             | Ctrl+Shift+N             |
| 关闭标签          | Cmd+W             | Ctrl+Shift+W             |
| 下一个/上一个标签 | Cmd+Shift+] / [   | Ctrl+PageDown / PageUp   |
| 打开设置          | Cmd+,             | Ctrl+,                   |
| 复制/粘贴         | Cmd+C / V         | Ctrl+Shift+C / V         |
| 搜索终端          | Cmd+F             | Ctrl+Shift+F             |
| 打开终端链接      | Cmd+主键单击      | Ctrl+主键单击            |
| 滚动到底部        | 未绑定            | 未绑定                   |
| 新建本地终端      | Cmd+L，动作未启用 | Ctrl+Shift+L，动作未启用 |

“全局”指应用窗口前台的application作用域，终端动作属于terminal作用域。Local工厂未实现，因此保留配置但不启动系统Shell旁路。SFTP目前使用状态栏开关，没有注册独立SFTP标签或面板快捷键。Windows/Linux默认Ctrl+C仍发送远端中断，Ctrl+L仍交给Shell；用户可显式改变绑定。

## 设置与安全边界

设置草稿支持按名称/绑定搜索、录制键盘组合、编辑链接鼠标修饰键和JSON导入导出。录制只捕获组合，不执行动作或发送远端输入。保存时校验已注册commandId、scope/context、修饰键与按键白名单，拒绝同作用域重叠绑定；不接受脚本、Shell命令或输入宏。当前平台字段仅支持all，不承诺跨平台独立覆盖或OS保留键检测。

路由顺序为录制/IME → 模态及输入控件/菜单上下文 → 终端动作 → 应用动作 → xterm默认编码。仅实际消费的动作阻止默认行为；模态/取色/选择浮层期间不抢焦点，文本输入保留系统复制粘贴。配置随设置保存，不注册OS后台热键。

macOS仅在无其他修饰键、无IME/AltGraph/活动弹层时，将Option+左右方向键转换为ESC b / ESC f；自定义快捷键优先。不开启全局macOptionIsMeta，远端行编辑器决定最终跳词行为。

## 终端链接

自动URL与OSC 8链接共用修饰键策略：普通点击/拖动保持选区和远端鼠标语义，符合配置的主键点击才请求打开。Rust再次校验http/https、长度与控制字符，拒绝file/javascript等协议；远端输出不得自动打开链接。

xterm在mouseup激活链接，处理程序仅preventDefault，不stopPropagation，保证document选区监听先完成清理；下一轮任务调用受控opener，已释放终端不再执行延迟打开。桌面实测需覆盖选区、鼠标上报、IME/AltGr、弹层优先级和一次动作只执行一次，见 [验证方案](11-validation.md)。

系统后台模式仅为后续范围，候选接口见 [Tauri Global Shortcut](https://v2.tauri.app/plugin/global-shortcut/)，当前未安装或注册。

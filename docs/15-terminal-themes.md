# 终端配色

终端配色与应用深色/浅色/跟随系统模式独立。以下能力已落源码，热切换、选区和三平台视觉效果待运行验收。

## 方案与引用

内置64套只读方案，默认 Pro；名称、色值与来源摘要见 [资源说明](../resources/README.md)。前后端共用 `resources/terminal-palettes.json`，不维护两份内置名单。自定义方案最多64套，编辑内置方案会创建副本。

应用优先级为当前 terminalId 临时覆盖 → 主机覆盖 → 全局默认。临时覆盖仅驻内存，重连保留；主机引用与自定义方案随设置保存。删除自定义方案前确认，相关主机引用回到全局默认；删除全局默认时回到 Pro。

设置页使用可搜索的自适应网格，随正文统一滚动；点击方案修改草稿。自定义编辑默认折叠，展开后创建独立样本终端，收起保留预览实例。保存自定义后选中该方案，最终持久化仍需点击底部“保存设置”。预览不向 SSH 发送命令。

## 颜色与渲染

可编辑前景、背景、光标、光标文字、活动/失焦选区、16个 ANSI 色、16–255索引覆盖和滚动条颜色。使用 Naive UI 取色器、HEX/RGB 输入，索引覆盖通过 HEX JSON 编辑。背景和 ANSI 基础色必须不透明；允许透明的字段使用 HEX8。只接受颜色白名单，不允许 CSS 表达式、脚本或资源 URL。

`src/modules/settings/themes.ts` 将方案转换为锁定 xterm 版本的 ITheme；索引覆盖映射为 extendedAnsi。远端显式 truecolor 不会随 ANSI 色表重着色。实际终端与预览均设置 `drawBoldTextInBrightColors=false`，SGR 1只改变字重，显式亮色仍使用亮色表。

主题更新应用于现有实例，不 clear/reset/dispose、不重连；舞台和宿主背景使用同一优先级解析，避免边缘露出应用底色。字体变化才按实际尺寸重新适配行列。

选区由 `src/modules/terminal/theme.ts` 统一适配：没有显式 selectionForeground 时，以原选区色覆盖现有选择矩形，使原文字参与透明混合；显式设置文字色则使用原生主题语义。不修改选择范围或复制内容。该适配依赖固定 xterm 6 的 `.xterm-screen/.xterm-selection` DOM和层级，升级或更换渲染器必须复验。

## JSON导入导出

支持单个 Termvexa Scheme v1 JSON与基础 xterm ITheme颜色对象，缺失的 xterm字段以 Pro为基底补齐。文件上限1 MiB；校验版本、16项 ANSI、颜色及16–255索引范围，导入生成本地随机ID。JSON解析使用受限解析器，拒绝重复键和非法结构。

导入文件先进入编辑预览，确认后作为新方案加入设置草稿；重名追加“导入副本”，不覆盖内置或现有方案。导出当前方案JSON，不包含主机、凭据、输出或历史。未实现批量导入、Windows Terminal/iTerm格式、主题文件拖入、通用撤销或对比度辅助。

## 验证入口

重点验证全局/主机/临时优先级、原终端热切换不丢历史、JSON往返与非法/超限拒绝、删除引用回退、透明选区及中英文/emoji预览。终端右键配色菜单使用 Naive UI滚动与视口约束。静态检查不能代替WebView视觉验收，见 [验证方案](11-validation.md) 与 [实施状态](17-implementation-status.md)。

API参考：[xterm ITheme](https://xtermjs.org/docs/api/terminal/interfaces/itheme/)，实际支持字段以锁定依赖和本项目映射为准。

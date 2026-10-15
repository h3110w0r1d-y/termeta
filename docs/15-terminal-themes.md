# Terminal 配色方案、编辑与导入

日期：2026-10-15。M2/P0 的64套提取预设、颜色编辑/JSON、三级引用与热应用已落源码，未运行/视觉验收。终端配色独立于应用界面“系统/深色/浅色”模式；用户可以使用浅色 UI + 深色终端或相反组合。

## 1. 内置方案与作用范围

按用户要求移除原8套项目预设，使用指定 Termius 解包文件静态提取的64套方案，包含 Nord Dark/Light、Solarized、Dracula、Gruvbox、Catppuccin 等。默认 Pro，保留原名称和终端色值；来源、SHA-256 与映射见 [配色资源说明](../resources/README.md)。前后端从同一 JSON 读取名单；提取仅解析字面量，不执行解包代码。旧预设明暗引用在读取时归一为 Nord Dark/Light，同名 Ocean 使用新的提取值；用户自定义方案保留。运行视觉、选区和 ANSI 渲染仍待验证。

方案列表支持预览、搜索、设为默认、复制为自定义、删除自定义、导入/导出。内置方案只读，编辑时创建副本；用户副本不会被升级覆盖。

应用顺序：当前 terminalId 的临时选择 → 主机/连接 profile 的 scheme 引用 → 全局终端默认方案。原终端重连保留临时选择；用户明确清除临时覆盖后回到 profile/default。首期 Local/Serial 未实现不妨碍 SSH 使用方案，未来共享同一主题模型。

删除正在引用的自定义方案前展示影响范围，允许改为指定替代方案再删除，不留下悬空引用；当前终端已显示内容仍保留，仅视觉更新。工作区持久恢复只保存 themeId/覆盖引用，不保存输出正文。

## 2. 完全自定义的颜色编辑器

| 颜色类别 | 可编辑内容 |
| --- | --- |
| 基础 | 前景、背景、光标、块光标文字 |
| 选择 | 活动选择背景/文字、失焦选择背景 |
| ANSI 基础 | 8 个常规色 + 8 个亮色，逐格修改 |
| 扩展调色板 | 16–255 索引色覆盖（按锁定 xterm release 的能力映射） |
| 辅助颜色 | 滚动条 normal/hover/active、可用的 ruler 色等 |

每个颜色提供自定义取色器、色块、HEX 输入、RGB 输入（支持的透明色另有 alpha）。可从任意合法颜色值选取，不限制在内置 palette；点击色块打开编辑。首期的“取色”指控件内任意颜色选择与手输数值，系统屏幕吸管为能力可用时的增强，不将浏览器 EyeDropper 在所有 WebView 可用作为前提。

内部颜色规范化为 #RRGGBB；允许透明的选择/辅助颜色可用 #RRGGBBAA。默认背景保持不透明；不因接受 RGBA 就声称支持窗口透明效果。颜色字段按类型约束，禁止 CSS url()/var()/任意表达式、外部资源、脚本和非颜色字符串。

实时预览含普通文本、16/256 色样本、显式 RGB、选区、光标、中文/emoji 与明暗背景。提供撤销/取消/保存/重置单色和复制方案；预览用独立受控样本终端，不向真实 SSH 发送测试命令或修改远端状态。对比度不足提示具体颜色组合，但不擅自更改用户选色；最低对比度增强设置单独说明可能影响实际显示。

ANSI/索引色可跟随 scheme 改变，远端程序显式输出的 truecolor RGB 不保证会被映射为自定义 ANSI 色；不能把“完全自定义配色”误写成会重着色一切远端显式 RGB。扩展 palette 字段必须核实具体 xterm release；不支持时提示能力限制或选择支持的版本，不静默忽略已编辑项。

## 3. 导入与导出

| 格式 | 计划范围 | 行为 |
| --- | --- | --- |
| Termvexa scheme JSON v1 | M2/P0 | 完整往返；包含版本、名称、基础色、ANSI 与扩展覆盖 |
| xterm ITheme JSON | M2/P0 | 按颜色白名单转换，缺项使用明确的基底方案并显示补全项 |
| Windows Terminal 单个 scheme/选定 schemes 数组 JSON | M3/P1 | 仅转换配色字段，不导入 shell/命令/快捷键或整套 settings |
| iTerm `.itermcolors` plist | M3/P1 | 安全解析选定颜色字段；不执行 plist 关联动作，不解析外部实体 |

首期将支持把用户方案转换为标准 JSON 后导入；其他格式按上表逐步支持，UI 明示支持格式，不能对任意文件称“主题导入成功”。

导入通过用户选择文件/主题设置区域的明确拖入，先解析校验，再展示名称、颜色、缺项/不支持字段、重名处理和预览，确认后持久化。默认不自动应用，也不覆盖同名现有方案。用户可选另存副本或明确替换自己的方案；不能改写内置资源。

初始文件大小上限 1 MiB、单批最多 100 方案，限制字符串长度、数组项数和嵌套深度。拒绝非法颜色、重复 JSON key 的歧义、未来 schema、非颜色代码；不解压任意主题压缩包、不访问文件内 URL。导出只含配色与允许的来源信息，无主机、凭据、输出/历史或可执行配置。

## 4. 版本化模型

```ts
interface TerminalColorSchemeV1 {
  schemaVersion: 1;
  id: string;
  name: string;
  source: "builtin" | "user" | "imported";
  colors: {
    foreground: string; background: string;
    cursor: string; cursorAccent: string;
    selectionBackground: string;
    selectionForeground?: string;
    selectionInactiveBackground: string;
    ansi: string[]; // 恰好 16 项
    indexedOverrides?: Record<string, string>; // 仅 16..255 的索引
    scrollbar?: { normal?: string; hover?: string; active?: string };
  };
}
```

导入文件的 id 不能直接成为可信数据库主键，后端分配本地 id；替换走明确目标 id + revision。颜色编辑器与 schema 统一，不让一部分 palette 只存 CSS、另一部分只存 xterm options。

## 5. 架构与持久化

`TerminalThemeService` 解析全局/profile/terminal 选择，转换为锁定版本的 xterm ITheme；应用 UI CSS tokens 由另一套模型管理，允许用户选择“终端跟随系统明暗方案”策略，但不强制两者绑定。

`ThemeRepository` 在 Rust storage 管理自定义方案与引用；内置方案随应用以受版本管理资源交付。设置保存默认 schemeId，profile 保存可选 schemeId，前端 terminalId 保存临时选择。导入由受控命令校验与持久化，前端不能任意写资源目录。

通过已存在实例的 theme/options 更新即时应用，不能 dispose/reset/clear，不能因更换主题重新连接或清空滚屏。隐藏终端也更新主题状态，显示后按需刷新；颜色变化不必发送远端 resize，字体/字号变化才按实际尺寸触发。

终端舞台与宿主边缘背景共用当前活动终端的配色解析，包含当前标签、主机覆盖和全局默认的相同优先级；切换标签或热修改配色时同步变化，避免边缘显示应用主题底色。

## 6. 预研依据与验收

官方 ITheme 提供背景/前景、光标、选择、ANSI 与扩展颜色字段；项目统一模型需按最终锁定 release 映射，当前页面不等于已完成集成。[xterm ITheme](https://xtermjs.org/docs/api/terminal/interfaces/itheme/)

验收：至少 8 个内置方案；任意自选颜色和 ANSI 逐格编辑；原终端热切换不清历史；全局/profile/临时优先级；JSON 导入预览/重名/无效颜色/超限拒绝；导出再导入保持颜色；删除引用方案有明确替代；应用主题与终端方案可独立设置。新增 SP-09 验证，详见验证计划。

## 首期实现映射（2026-10-15）

resources/terminal-palettes.json 保存64套提取配色，src/modules/settings/themes.ts 提供xterm映射，ThemeSettings/ColorField实现编辑/取色/HEX/RGB/JSON。app设置中hostSchemes保存主机覆盖，当前terminal覆盖仅内存；用户自定义方案保留上限64（与64套内置配色分别计算）。背景/ANSI使用HEX6，选择色可带透明度；运行热更新/对比度/传递字段仍须SP-09验证。 具体交付及限制见 [首期实施状态](17-implementation-status.md)。

配色选项采用名称在上、ANSI 色块在下的两行布局，由 Naive UI NFlex 组织按钮内容；按钮高度随内容扩展，色块作为装饰不重复朗读。视觉效果待运行复验。

64项配色采用 NGrid 自适应网格平铺，不限制高度、不设独立滚动，随设置正文的 NScrollbar 统一滚动。搜索与导入位于网格上方；点击卡片修改全局配色草稿并显示选中标记。自定义编辑默认折叠于网格下方，使用 NCollapse，展开后才初始化 xterm 预览，收起后保留实例；编辑区末尾为保存为自定义按钮，保存后选中该方案。设置正文与底部保存设置操作独立，最终持久化仍需保存设置。

粗体颜色语义（2026-10-15）：实际终端与配色预览显式设置 drawBoldTextInBrightColors=false。SGR 1 只改变字重，不将普通 ANSI 0–7 提升为8–15；显式高亮色仍使用高亮色表，256色和真彩色不变。Pro 等方案色值不修改，避免用改色表补偿渲染策略。

选区合成适配（2026-10-15）：实际终端与预览统一通过 src/modules/terminal/theme.ts 的 applyTerminalTheme 应用主题。未设置 selectionForeground 时，xterm 选区主题临时设为透明，避免其 DOM 渲染器为选中文字绘制已混合的底色；业务 CSS 使用原 selectionBackground/selectionInactiveBackground 将原有选区矩形覆盖在文字上方，使 ANSI/真彩色文字也参与透明混合。颜色不写回 Scheme，不改选择范围/鼠标/复制，也不逐字监听或重绘。不透明选区色沿用 xterm 的30%透明度；显式 selectionForeground 关闭该适配，使用原生主题语义。适配依赖固定 xterm 6 的 .xterm-screen/.xterm-selection DOM 和层级，升级或换渲染器须复验，不保证与 Termius 每个像素一致。

终端右键配色列表（2026-10-15）：64套内置与自定义方案仍由Naive UI子菜单展示，启用库scrollable，主/子菜单通过menu-props共用视口限高/限宽，靠边由库flip定位；长名称省略，不再按全部选项撑开到窗口外。

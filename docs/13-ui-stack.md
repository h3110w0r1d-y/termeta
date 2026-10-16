# UI技术选型与桌面适配

采用固定版本 **Naive UI 2.45.3** 与Vue 3。工作区标签使用自定义WorkspaceTabs，其余有对应能力的交互控件优先使用组件库；项目封装只适配业务值、事件、标签和安全默认值，不重造弹层定位或控件皮肤。当前源码待三平台WebView运行验收。

应用主题色支持在设置常规页通过 NColorPicker 选择不透明六位 HEX，并恢复 Naive UI 默认色；有效修改自动保存并跨窗口热应用。`app.primaryColor` 缺失或空串沿用库的明暗主题默认色，经 Rust 白名单及颜色校验，不修改 schema。主窗口和设置窗口共用 NConfigProvider themeOverrides 派生主色/悬停/按下状态，业务 accent 同步主色；Workspace 拖放预览边框读取 useThemeVars.primaryColor，填充为该色25%透明度，不影响终端配色或终端宿主生命周期。仅完成前端类型、结构、Vue格式与差异静态检查；未构建、启动或执行 Rust 编译/测试，三平台视觉与设置同步待运行验收。

终端配色卡片新增 SchemeThumbnail 轻量DOM缩略图，展示普通文字横条、选区及其文字、绿色文字、光标和常显6px ANSI六色条；72×48px布局按用户提供的Termius解包ThemeItem/useStyles$3o适配为Vue scoped样式，不执行或引入原React代码。内置/自定义/导入方案共用现有Scheme数据，显式selectionForeground优先；外层按钮的选中/悬停沿用Naive UI主题，点击自动保存及独立xterm编辑预览不变。仅类型、格式、结构与差异静态检查，未构建或启动，视觉待运行验收。

## 组件与外观

`NConfigProvider`、darkTheme、中文locale和NGlobalStyle管理应用外观，应用主题与终端配色独立。CSS只保留业务布局、尺寸与终端适配，不用全局button/input规则改写库内部DOM；定制优先使用公开props或themeOverrides。

| 场景                        | 当前组件与边界                                                                 |
| --------------------------- | ------------------------------------------------------------------------------ |
| 编辑/认证/确认弹窗          | AppDialog薄适配NModal card；content-scrollable正文与footer分离，操作区始终可见 |
| 按钮、文本/数字输入、复选框 | NButton、NInput、NInputNumber、NCheckbox；保留表单、IME及值类型                |
| 下拉与菜单                  | NSelect、NMenu、NDropdown；空值与业务“跟随默认”等选项显式映射                  |
| 配色与折叠                  | NColorPicker、NCollapse；颜色校验与xterm预览由业务负责                         |
| 页面与设置滚动              | NScrollbar；xterm保留自身滚动及背压                                            |
| 文件列表与任务              | NDataTable、库复选框/进度/确认控件；本地文件授权仍由原生桥管理                 |
| 系统文件选择/拖放           | Tauri原生对话框/事件；macOS拖出使用文件承诺，不能用网页DnD代替                 |

库没有独立Label时保留HTML标签关联；语义布局、form/fieldset和普通文本无需组件化。配色JSON的隐藏文件输入只作明确选择桥，不用于读取SSH秘密。

## 共用滚动渐隐

`components/ui/ScrollFade.vue`为滚动容器的薄装饰，主机列表、分组树和工作区标签复用；内部滚动继续由NScrollbar或原生DOM负责，不创建第二个滚动容器。`background`接收所在区域的CSS颜色/变量（默认`var(--bg)`），`size`为渐隐像素长度（默认20，非负且最多占当前方向的20%），`direction`为vertical（默认）或horizontal。主机页两栏传`var(--bg)`，Tab栏传`var(--surface)`，主题变化直接跟随CSS变量，不混用底色。

对NScrollbar默认在插槽内查找`.n-scrollbar-container/.n-scrollbar-content`；锁定2.45.3公开实例只有scrollTo/scrollBy，不依赖内部实例ref，该DOM约定与已有主机拖动适配一致，升级库须复核。原生滚动节点传`viewport`，必要时可显式传`content`。外层只需承担布局尺寸；内容增删/标题变化由MutationObserver、viewport/内容尺寸和字体变化由ResizeObserver触发，scroll被动监听并按动画帧合并测量。无溢出、不可见或到达边缘时移除对应端渐隐，小数及弹性滚动容差1px；卸载清理观察器/监听/动画帧。

渐隐用外层伪元素绘制背景色至透明渐变，pointer-events:none，无额外焦点或无障碍节点。固定标题/分组开关、固定标签与新建按钮不包含在渐隐区，滚轮、键盘滚入可见、滚动条及拖动边缘自动滚动沿用实际viewport。useTabScroll只保留顶部纵轮转横向，不重复测量边缘。

```vue
<ScrollFade background="var(--bg)" :size="20">
  <NScrollbar>…</NScrollbar>
</ScrollFade>
```

仅类型、结构、格式与差异等静态检查；未构建或启动，三平台明暗主题、滚动边界/列表与树变化、隐藏恢复和拖动命中待运行验收。

## 图标来源与替换范围

通用 UI 图标统一使用 xicons 的 Vue 3 包 `@vicons/ionicons5` 0.13.0，通过 NIcon 保持尺寸、颜色与按钮可访问名称。按用户最新要求，保险箱等8个按钮/导航图标已移除Termius SVG，AssetIcon保留语义名称并静态映射到Ionicons；SSH主机系统/类型图标完整保留，不接收运行时外部SVG。装饰图标不参与无障碍名称。

| AssetIcon名称 | Ionicons组件 | 引用位置 |
| --- | --- | --- |
| add | AddOutline | 标签栏新建标签 |
| sftp | FolderOpenOutline | 固定SFTP标签 |
| hosts | ServerOutline | 保险箱主机导航 |
| vault | LockClosedOutline | 固定保险箱标签 |
| credentials | KeyOutline | 凭据导航 |
| local-terminal | TerminalOutline | 主机页本地终端按钮 |
| host-trust | ShieldCheckmarkOutline | 主机信任导航 |
| connection-history | TimeOutline | 连接历史导航 |

无新增依赖。SFTP文件类型及主机分组文件夹保留现有自绘图案；系统原生窗口图标、品牌资源和Naive UI/xterm内部图标不在替换范围内。SSH卡片继续使用HostSystemBadge及原28个系统/类型SVG；串口使用HardwareChipOutline。页面favicon沿用带Ionicons MIT声明的TerminalOutline静态SVG。

主机页新建主机与快速连接、NewTabDialog快速连接共用ConnectionTypeDropdown薄适配NDropdown/NButton，统一placement="bottom-start"从按钮下方起始边对齐，悬浮显示SSH/Telnet/串口，Telnet已启用。显式处理外部点击、Tab、Escape及窗口失焦关闭，页面隐藏时同步关闭；新建标签页搜索在类型菜单展开时让菜单处理方向键与Enter，退场后再打开所选快速连接表单。Telnet与SSH共用HostEditor的Naive UI表单布局，Telnet隐藏SSH认证字段并显示明文说明；不增加另一套表单控件。

仅静态检查通过，三平台图标尺寸、对齐、主题颜色与菜单键盘/IME交互仍待运行验收；不报告构建体积。

## 样式归属

Vue 模板由现有 Prettier 3.9.10 统一格式化，仓库 `.prettierrc.json` 对 `*.vue` 设置 `htmlWhitespaceSensitivity: "ignore"`，避免为保留行内空白而拆开闭合标签或连接相邻组件；多行正文与闭合标签自然分行，短元素允许单行。`pnpm format:vue` 批量修复，`pnpm check:format:vue` 仅静态校验。精确依赖行内边界空格的内容使用显式文本表达式或局部 `prettier-ignore`，预格式化内容保持原样。此次批量整理不修改脚本逻辑、样式、接口或依赖；行内文本边界的视觉效果仍待运行复验。

2026-10-16 将原集中式 `src/styles.css` 拆入对应 Vue 的 `<style scoped>`：WorkspaceTabs 维护标签/排序及横纵布局，TerminalStage 维护分屏容器/分隔条，TerminalPane 维护窗格与 xterm 宿主，窗口按钮、弹窗及各设置页维护自己的布局。标签组件按 position/isMac 在自身根节点表达布局状态，避免依赖父组件的 scoped 选择器。对 xterm 动态节点和跨子组件的调整仅用明确的 `:deep()`。

`App.vue` 的 scoped 样式管理应用外壳/保险箱/状态栏；非 scoped 样式只保留主题 tokens、基础规则、共享表单/页头/状态点以及同时用于实际终端与预览的 xterm 选区、传送菜单适配。字体资源 CSS 独立保留。已删除 `src/styles.css` 及 main.ts 导入，并清理没有模板引用或始终被覆盖的旧规则。此轮仅静态检查，视觉、横纵布局、小窗口和弹层/xterm 仍待运行复验。

## 生命周期与焦点

- WorkspaceStore是唯一标签状态，固定保险箱/SFTP不创建session。TerminalPane按terminalId保活，显隐、排序、分屏和重连不重建或清空xterm。
- WorkspaceTabs采用单一tablist、roving tabindex、方向键/Home/End/Enter/空格、Delete关闭及菜单键；固定页与会话滚动区分离。标签主题色和圆角读取useThemeVars，拖动与分屏几何见 [工作区](18-terminal-workspaces.md)。
- 保险箱主机/凭据各有NScrollbar，独立设置窗口使用NScrollbar且自动应用状态footer独立；主机页v-show挂原生容器，避免滚动组件观察器根元素破坏显隐。凭据/设置共享NH1/NText页头，主机页搜索置顶。
- 配色网格自然展开，随设置正文滚动；自定义编辑默认折叠，展开后才创建预览。字体下拉固定12项，选项及当前值以对应本地字体显示，资源见 [字体清单](../public/fonts/README.md)。
- 活动模态、选择/取色浮层优先处理焦点与Escape；关闭动画保留DOM时，AppDialog活动标记已经移除，避免确认后的粘贴被自身动画拦截。
- 无关界面文字默认不可选；输入、可复制正文和xterm保留选择能力，不添加全局选择/鼠标拦截。

## 安全与认证

VaultGate使用Naive UI的NCard/NForm/NInput/NButton/NSpin/NAlert，先完成首次密码设置或自动/密码解锁再挂载业务页面；输入提交后清空，系统主密钥缓存失败显示警告。本轮仅静态检查，首次/恢复焦点和密码输入体验待运行验收。


SSH主机/快速连接提供密码和公钥认证；OTP/PAM/Keyboard-interactive由服务端协商动态触发。凭据仅密码/私钥，私钥可带解密密码；主机专属密码非空自动保存、显式空值改为连接时询问，OTP不保存。认证挑战不能因浮层Escape或焦点变化被意外提交/取消。

私钥及文件正文字节在Rust读取；前端不获得任意本地路径读写能力。保留CSP既定style-src范围，不为组件库放宽script-src、导航或IPC所有权检查。SFTP拖放与限制见 [SFTP设计](16-sftp-workspace.md)。

## 依赖与验证

已核对2.45.3包manifest、类型和许可证：Node要求>=20，Vue peer为^3.0.0，MIT。包声明sideEffects:false，未构建因此不报告实际体积或tree-shaking效果。来源：[官网](https://www.naiveui.com/)、[仓库](https://github.com/tusen-ai/naive-ui)、[固定版本元数据](https://registry.npmjs.org/naive-ui/2.45.3)。API以锁定包源码为准。

待验收：三平台高DPI、系统主题、弹层焦点/IME、表单Enter/校验、横纵标签、隐藏终端、文件选择/拖放和颜色往返。类型、格式及静态契约检查不代表桌面交互已通过，见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。

业务表单使用NForm/NFormItem的model/rules/path和validate；由required规则生成红星，提交按钮不因输入未填而提前禁用，提交时显示字段错误。自定义配色与常规设置使用独立表单，禁止嵌套HTML form。主机搜索采用NSelect的remote/filterable与header插槽，顶部字段复选框共用即时搜索设置。

NSelect.focus仅聚焦选择器外壳，主机搜索通过focusInput获取真实光标。搜索字段复选框mousedown阻止默认焦点转移；有效关键词为空时菜单隐藏。AppDialog转发Naive UI after-leave，供连接/认证弹窗完成焦点陷阱退场后恢复终端。搜索和上下文菜单使用display-directive=show，并让data-ui-popup-open随真实显示状态变化，防止过渡期间的旧节点阻断快捷键。

全局NewTabDialog复用AppDialog、NInput、NScrollbar、NList/NButton和SearchPreferences，主机结果直接位于正文，无第二层下拉；共享useHostSearch保证与主页字段/拼音语义一致。AppDialog转发after-enter供真实输入聚焦、after-leave供弹窗串行交接，footer提供快速连接入口。

独立设置窗口使用原生标题栏，Windows/Linux主窗口应用菜单使用NDropdown/NButton；macOS使用Tauri原生应用菜单。公共主题/基础CSS由assets/base.css共享，两个窗口均使用NConfigProvider及相同Naive主题。普通设置和自定义配色编辑经表单校验自动应用，无显式保存按钮；JSON导入/删除仍保留业务确认。

## 表单响应式模型与校验时序

沿用 [Naive UI Form 官方示例](https://www.naiveui.com/zh-CN/os-theme/components/form) 的 model/rules/path/v-model 绑定方式：NForm 持有响应式状态，控件直接写入其字段，提交前 validate。已有分散 ref 通过 reactive 保留引用，作为同一个 model 暴露，不复制字段值；重开草稿、密码清理和配色替换仍更新原 ref，表单同步读取最新状态。主机草稿及配色对象使用嵌套路径，并同步规则与字段 path。

Naive UI 2.45.3 的 NSelect 更新回调结束后立即触发 change/input；FormItem 此时从 NForm 当前 model 按 path 取值，不会自动等父组件重渲染。临时对象或 computed 对象快照会导致校验取旧值，错误也不会仅因下次渲染更新 model 自动消除。因此禁止将这类快照作为 model；下拉弹层的快捷键标记与表单校验分离，不以焦点或 nextTick 补丁修正数据源。

本次统一 8 个组件的 9 张表单：主机编辑、凭据编辑、分组编辑、移动主机、保险库、双栏 SFTP 主机选择与路径、侧栏 SFTP 路径、自定义配色。常规设置、认证回答、确认输入已有直接响应式对象或数组绑定，保留原实现。系统私钥文件导入、配色单色重置不经过输入控件事件，成功写入 model 后通过对应 FormItem.validate 重验，保留其他字段错误。

结构静态检查拒绝 NForm 的内联对象和 computed model 常见写法。类型/结构/格式/差异检查通过，未构建、启动或运行测试。待运行复验：空值首次选中及再次切换、错误后改为有效值、清空后再次出现错误、主机/凭据重开和认证切换、无效路径纠正、配色名称与颜色重置、系统私钥导入后错误清除、鼠标和键盘选择、秘密清理。

主机编辑布局使用 NGrid（双列、16px列距、不另加行距）与 NFormItemGi，跨列说明/密码块/收藏/错误由 NGi 承载，移除 host-fields/form-grid 与 field 的自定义 grid。避免原18px行距与 NFormItem 自带反馈区间距叠加；纵向表单间距沿用库默认。禁用状态继续由 NForm 和既有控件属性控制，提交与校验不变。仅静态检查，实际间距和窄窗口视觉待运行复验。

固定SFTP标签及新建标签图标现使用上述Ionicons映射。新建标签页加号取消此前14px的单独缩小，恢复20px，按钮尺寸沿用原值；仅静态检查，视觉待运行复验。

## 用户指定的SSH系统/类型图标例外

本轮按用户明确要求，28个Termius主机系统/类型SVG原样复制至src/assets/svg/systems，清单[host-system-icons.json](../resources/host-system-icons.json)保留原文件名、语义标识和SHA-256。类型包括检测器的发行版/系统以及SSH、Pi、AWS、DigitalOcean手动选项；raspbian/pi共用原pi素材、suse对应原suze素材。系统/类型SVG继续作为明确例外保留，原8个按钮/导航SVG已由Ionicons替换。

主机卡片的HostSystemBadge用NAvatar承载40×40px系统色底/12px圆角和24px白色Logo，不增加描边；系统色取解包reconnectSaga-e37b5571.css变量，Debian为#CE0056。HostSystemIcon用NIcon承载，静态白名单raw资源、逐Vue实例重写SVG内部ID和引用，避免多卡片/选择器裁切路径串用；图标为装饰，文字提供可访问名称。不接受动态外部素材。主机编辑用NSelect图文标签，设置用现有NCheckbox/NCollapse/NInput只读文本，保留库样式。静态摘要/主动内容检查通过，三平台视觉待验收。

2026-10-16：用户指定SSH默认项改用`host.react-ecf6b335.svg`，原样替换`src/assets/svg/systems/ssh.svg`并更新清单来源与SHA-256；id=ssh和背景/尺寸不变，卡片/列表/侧栏/选择器及未知图标回退共享该素材，其他系统图标保留。仅资源与格式/差异静态检查，未构建或启动，视觉待运行验收。

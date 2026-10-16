# UI技术选型与桌面适配

采用固定版本 **Naive UI 2.45.3** 与Vue 3。工作区标签使用自定义WorkspaceTabs，其余有对应能力的交互控件优先使用组件库；项目封装只适配业务值、事件、标签和安全默认值，不重造弹层定位或控件皮肤。当前源码待三平台WebView运行验收。

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


主机/快速连接提供密码和公钥认证；OTP/PAM/Keyboard-interactive由服务端协商动态触发。凭据仅密码/私钥，私钥可带解密密码；主机专属密码非空自动保存、显式空值改为连接时询问，OTP不保存。认证挑战不能因浮层Escape或焦点变化被意外提交/取消。

私钥及文件正文字节在Rust读取；前端不获得任意本地路径读写能力。保留CSP既定style-src范围，不为组件库放宽script-src、导航或IPC所有权检查。SFTP拖放与限制见 [SFTP设计](16-sftp-workspace.md)。

## 依赖与验证

已核对2.45.3包manifest、类型和许可证：Node要求>=20，Vue peer为^3.0.0，MIT。包声明sideEffects:false，未构建因此不报告实际体积或tree-shaking效果。来源：[官网](https://www.naiveui.com/)、[仓库](https://github.com/tusen-ai/naive-ui)、[固定版本元数据](https://registry.npmjs.org/naive-ui/2.45.3)。API以锁定包源码为准。

待验收：三平台高DPI、系统主题、弹层焦点/IME、表单Enter/校验、横纵标签、隐藏终端、文件选择/拖放和颜色往返。类型、格式及静态契约检查不代表桌面交互已通过，见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。

业务表单使用NForm/NFormItem的model/rules/path和validate；由required规则生成红星，提交按钮不因输入未填而提前禁用，提交时显示字段错误。自定义配色与常规设置使用独立表单，禁止嵌套HTML form。主机搜索采用NSelect的remote/filterable与header插槽，顶部字段复选框共用即时搜索设置。

NSelect.focus仅聚焦选择器外壳，主机搜索通过focusInput获取真实光标。搜索字段复选框mousedown阻止默认焦点转移；有效关键词为空时菜单隐藏。AppDialog转发Naive UI after-leave，供连接/认证弹窗完成焦点陷阱退场后恢复终端。搜索和上下文菜单使用display-directive=show，并让data-ui-popup-open随真实显示状态变化，防止过渡期间的旧节点阻断快捷键。

全局NewTabDialog复用AppDialog、NInput、NScrollbar、NList/NButton和SearchPreferences，主机结果直接位于正文，无第二层下拉；共享useHostSearch保证与主页字段/拼音语义一致。AppDialog转发after-enter供真实输入聚焦、after-leave供弹窗串行交接，footer提供快速连接入口。

独立设置窗口使用原生标题栏，Windows/Linux主窗口应用菜单使用NDropdown/NButton；macOS使用Tauri原生应用菜单。公共主题/基础CSS由assets/base.css共享，两个窗口均使用NConfigProvider及相同Naive主题。普通设置和自定义配色编辑经表单校验自动应用，无显式保存按钮；JSON导入/删除仍保留业务确认。

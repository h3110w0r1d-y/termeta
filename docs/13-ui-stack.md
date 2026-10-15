# UI 技术选型与 Naive UI 桌面适配

更新日期：2026-10-15。当前采用 **Naive UI 2.45.3**，替代此前 Reka UI 方案。迁移源码待集成验证；未构建、启动应用或执行三平台 WebView 验收。

## 1. 选型与实施原则

用户明确要求放弃 Reka UI，全部切换到 Naive UI，并尽量使用组件库现成外观。`naive-ui` 固定 2.45.3，移除 `reka-ui`；不并存两套 UI 库。优先直接使用 Naive UI 组件，必要的项目封装只适配业务值、事件、可访问标签和安全默认值，不重写弹层定位、选项导航或取色算法。

外观由 `NConfigProvider`、`darkTheme`、中文 locale 和 `NGlobalStyle` 统一管理。应用深色/浅色/跟随系统与终端配色独立。保留布局、尺寸和终端宿主所需 CSS，删除原生选择器箭头、按钮/输入框/复选框皮肤及页面重复控件样式；不得用全局 `button/input` 规则覆盖库内部 DOM。确需定制优先采用公开 props、themeOverrides 或组件公开样式接口。

## 2. 固定版本核查

已核对 npm 元数据、已安装包 manifest、MIT LICENSE 和 2.45.3 包内 `.d.ts` / `.mjs`：Node 要求 `>=20`，Vue peer 为 `^3.0.0`，与项目 Vue 3 方向一致。按需导入组件，包声明 `sideEffects:false`；未经构建不报告实际 tree-shaking、体积或性能收益。

来源：[官网](https://www.naiveui.com/)、[官方仓库](https://github.com/tusen-ai/naive-ui)、[固定版本注册信息](https://registry.npmjs.org/naive-ui/2.45.3)。实际 API 以锁定包源码为准，不能把官网后续变动当作已安装版本行为。

## 3. 控件映射

| 部位 | 当前组件 | 应用边界 |
| --- | --- | --- |
| 主机/认证/信任/确认弹窗 | `NModal` card preset、`content-scrollable`、`#footer` | AppDialog 统一开关和固定操作区，正文独立滚动；锁定挑战只能明确拒绝/取消 |
| 所有下拉选择 | `NSelect` | AppSelect 适配 null/undefined/空串与字符串值，选项只含非秘密元数据 |
| 普通按钮、文本与多行输入 | `NButton`、`NInput` | 薄封装明确 button/submit 类型、IME、字段属性和禁用状态 |
| 复选框与数字 | `NCheckbox`、`NInputNumber` | 保留布尔和数值契约，不能因替换而静默提交空值 |
| 应用/设置标签与分类 | 应用工作区使用自定义 `WorkspaceTabs`；设置仍使用 `NTabs` / `NTabPane`、`NRadioGroup` | 工作区固定区与会话滚动区共用单一 tablist，终端内容独立保活 |
| 首页侧栏菜单 | `NMenu` | 只切主机/凭据/设置，不能自动连接服务器 |
| 配色编辑与高级折叠 | `NColorPicker`、`NCollapse` | 使用库取色、HEX/RGB；方案校验和 xterm 预览仍为业务逻辑 |
| 内容滚动 | `NScrollbar` | xterm 自身滚动与输出背压不替换、不包入通用文本控件 |
| 系统文件选择 | Tauri 原生对话框；JSON 导入的文件选择桥 | 库不能替代系统文件授权，文件内容不自动上传；私钥始终只在 Rust 读取 |

语义布局容器、form/fieldset、非交互文本不是需要替换的控件。Naive UI 无独立 Label 时保留正确的字段标签关联；不为追求“无 HTML”制造无意义组件。文件输入可作为不可见的浏览器选择桥，按钮和可见入口使用组件库。

## 4. 生命周期、焦点与安全

AppDialog 使用 NModal card 预设的 `content-scrollable` 与受视口约束的最大高度；`#footer` 原样转发库 footer，操作区与正文分离，使用 NFlex 排列按钮，不自写滚动弹窗布局。主机、凭据、认证、确认、粘贴和设置均遵循此契约；外置 submit 按钮通过 form ID 关联表单。设置内嵌模式保留相同插槽但不创建模态层。

CSS 默认限制无关文字选择，通过 input/textarea/contenteditable、可复制正文与 `selectable-text` 开放实际内容；保留 xterm 自带选择机制和可访问性选择层，不添加全局鼠标/选择事件拦截。

- `WorkspaceStore` 是唯一标签状态；固定首页/SFTP 不创建 session，关闭按钮只作用于 SSH 标签。固定区不随会话溢出滚动消失。
- `TerminalPane` 始终按 terminalId 挂载，使用可见性切换；不把终端交给 TabPane 的默认卸载策略。切库、布局、认证失败、断线或重连不 clear/reset/dispose。
- 工作区标签使用自定义原生按钮与布局，支持单一 tablist、roving tabindex、方向键/Home/End/Enter/空格、Delete 关闭、菜单键/Shift+F10 与焦点衔接。横向/纵向会话区独立溢出滚动；固定首页/SFTP 不关闭且不随会话滚动。菜单弹层继续使用 NDropdown，终端内容不由标签组件挂载。拖拽排序留后续实现，不引入依赖或第二套标签状态。
- 模态与选择/取色浮层打开期间，应用快捷键与延迟终端聚焦让位于控件；Escape 先关闭内层浮层，不能意外取消认证或关闭主机编辑。
- NSelect 的空值与“连接时询问/跟随默认/未分组”显式映射，不把占位符当作用户选择；异步选项更新不能改变业务引用。
- CSS-in-JS 与 xterm 动态样式沿用已明确记录的 `style-src 'unsafe-inline'` 范围，不放宽 script-src、导航和 IPC ACL。

## 5. 认证界面约定

主机和快速连接只有“密码”和“公钥认证”两项；公钥认证通过授权的 OpenSSH 私钥完成签名。OTP/PAM/Keyboard-interactive 只由后端根据服务器 offered methods / partial_success 动态触发，不能作为第三个可选认证方式或追加开关。

凭据库仅密码/私钥，私钥可附解密密码；主机可引用公共凭据或直接输入专属密码，非空自动保存、空则连接时询问，不提供记住开关。旧 keyboardInteractive/独立 passphrase 配置不保留兼容；OTP 不自动填充或保存。

## 6. 验证边界

本轮允许执行 `vue-tsc --noEmit`、Prettier、Rust 格式、依赖公告和静态契约检查；不构建运行。SP-07~09 仍需验证：Modal/Select/ColorPicker 叠层与焦点、系统主题变化、三平台高 DPI、Tab 横纵布局与隐藏终端、IME/快捷键、表单 Enter/校验、文件导入、透明颜色和配置往返。具体结果见 [实施状态](17-implementation-status.md)。

终端字体使用既有 AppSelect → NSelect 固定12项选择，取消自由文本输入；字型资源、许可证和版本清单置于 `public/fonts/` 并随应用分发。只添加字体声明 CSS，不自定义选择器皮肤；默认 JetBrains Mono，设置保存后异步加载所需字型并更新原 xterm。

字体选择预览使用 NSelect 原生 render-label（由 AppSelect 的 attrs 透传），菜单选项与当前值都以对应本地字体渲染名称；字体加载由 CSS Font Loading 按可见内容触发，不另造选择器或预览皮肤。

首页与设置滚动（2026-10-15）：AppDialog 内嵌模式用 NScrollbar 承载正文，#footer 在独立底部区；弹窗模式仍转发 NModal card footer。首页内容不再套统一大滚动条，主机/凭据各有 NScrollbar，设置填满剩余空间。配色选择使用 NGrid/NGridItem 自适应平铺（按容器宽度1–5列），网格自然展开，不设限高或独立滚动容器，随设置正文的 NScrollbar 统一滚动。搜索/导入在上方，自定义以 NCollapse 默认折叠于下方，内部末尾保存。三页统一 NH1/NText 与共用业务布局，不另造标题组件皮肤。

滚动组件显隐边界（2026-10-15）：NScrollbar 默认通过 ResizeObserver 包装根元素，业务 v-show 不直接挂 AppScrollArea/NScrollbar；首页主机页使用原生容器承接显隐，滚动组件在内部保活，防止切页后主机与其他页面同时占据布局。

工作区标签状态色（2026-10-15）：WorkspaceTabs 通过 useThemeVars 读取当前 NConfigProvider 主题，悬停用 buttonColor2、激活用 buttonColor2Hover、状态文字用 textColor1。激活规则优先于悬停规则，深色主题下激活比悬停更亮；浅色主题使用库对应的强调底色，不写死新色值。

标签尺寸与圆角补充（2026-10-15）：标签和关闭按钮圆角统一读取 useThemeVars().borderRadius；标签36px高，关闭按钮24px，按钮上下及右侧留白6px。关闭按钮清零原生 padding，以 flex 居中，NIcon/SVG 块级显示避免基线偏移。

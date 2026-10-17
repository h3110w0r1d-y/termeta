# UI 技术选型与桌面适配

更新：2026-10-17。采用 Naive UI 2.45.3 + Vue 3，应用标签使用自定义 AppTabs，其余有对应能力的控件优先组件库。封装只适配业务值、事件、标签和安全默认值，不重造皮肤/弹层定位；三平台 WebView 交互尚待完整验收。

## 组件与外观

NConfigProvider、darkTheme、随界面语言切换的 locale/dateLocale 和 NGlobalStyle 管理应用外观。应用明暗/主色与终端配色独立；主色为不透明六位 HEX，缺失/空串沿用库当前主题默认色。useAppAppearance 统一派生主色/悬停/按下与业务 accent，两窗口共享；Workspace 拖放预览读取主题主色。有效设置自动保存并跨窗口应用。

| 场景                        | 组件与职责                                                      |
| --------------------------- | --------------------------------------------------------------- |
| 编辑/认证/确认              | AppDialog 薄适配 NModal card，滚动正文与 footer 分离            |
| 按钮、文本/数字输入、复选框 | NButton、NInput、NInputNumber、NCheckbox，保留表单/IME/值类型   |
| 选择、菜单、树、列表        | NSelect/NAutoComplete、NMenu/NDropdown、NTree、NList/NDataTable |
| 配色/折叠/布局              | NColorPicker、NCollapse、NGrid/NFormItemGi/NGi                  |
| 页面/设置滚动               | NScrollbar；终端保持 xterm 自身滚动/背压                        |
| 系统文件与窗口              | Tauri 原生选择/事件/窗口命令；macOS 拖出使用原生文件承诺        |

CSS 仅保留业务布局、尺寸和终端适配，定制优先公开 props/themeOverrides，不用全局 button/input 规则改写库内部 DOM。普通语义文本、form/fieldset 与 HTML 标签不必封装。

配色卡片使用轻量 DOM SchemeThumbnail 展示文字、选区、光标与 ANSI 色，内置/自定义/导入共享 Scheme 数据；点击自动保存，编辑预览使用独立 xterm，不执行远端命令。详见 [配色](15-terminal-themes.md)。

## 界面语言与文案

`src/i18n/index.ts` 提供 Vue 响应式语言状态、纯文本翻译与日期格式化，`messages/` 按公共/主机/终端/设置归集英文资源，中文原文兼作简体中文资源与词条标识。当前只需纯文本与命名参数 `{name}`，沿用现有 Vue/Intl 能力，不新增依赖或 HTML/脚本解释；后续需要复数/复杂语言规则时再评估成熟国际化库。两个窗口的 NConfigProvider 同步控件 locale/dateLocale，页面根节点更新 lang。SettingsWindow 使用同一词条源同步 document.title 和原生窗口标题，串行发送避免旧请求覆盖新语言；创建阶段仅显示 Termeta，读取偏好后应用完整标题，失败在设置页显示错误。原生 setTitle 仅经 desktop 门面调用，settings capability 定向开放库的 allow-set-title，无新增自定义命令。

常规设置提供“跟随系统 / 简体中文 / English”，沿用现有加密设置与跨窗口失效通知。跟随系统按 navigator.languages 顺序选择首个中文或英文，未知语言回退 English，系统 languagechange 会重新计算。保险库解锁前无法读取加密偏好，先按系统语言显示，解锁后应用保存值；不额外在明文存储复制设置。

模板、按钮、菜单、辅助访问名称、表单校验与状态显示使用 t；选项和展示映射在 computed/函数中求值以便热切换。动态整句使用命名参数，用户名称、远端认证提示/终端输出、路径、配色名和协议标识保留原文。已经显示的错误、确认请求和写入终端缓冲的本地提示是当时语言的快照，切换不重写历史；新生成的内容使用新语言。后端返回的诊断、钥匙串警告以及系统原生菜单/对话框的应用文案本轮仍保留原文，不宣称完整应用全量本地化。

用户提示优先说明操作、结果与风险，移除只解释内部队列、脚本、存储和认证机制的告示；明文传输、信任密钥、覆盖/删除/取消和数据无法恢复等必要后果保留。系统识别页不再展示内部检测脚本。语言切换不得重连、重建 xterm 或翻译用户保存的数据。

`pnpm check:i18n` 只解析源码与词典，检查直接引用的英文词条、参数一致性和模板中文遗漏；它不导入业务、不构建或运行项目，不能证明动态键覆盖、语言切换或视觉排版正确。新增文案同时维护词典，并执行类型、结构及 Vue 格式静态检查。三平台热切换、跨窗口保存、锁定页回退、表单/弹层及英文长度仍待运行验收。

## 共用滚动渐隐

`components/ui/ScrollFade.vue`为滚动容器的薄装饰，主机列表、分组树和工作区标签复用；内部滚动继续由NScrollbar或原生DOM负责，不创建第二个滚动容器。`background`接收所在区域的CSS颜色/变量（默认`var(--bg)`），`size`为渐隐像素长度（默认20，非负且最多占当前方向的20%），`direction`为vertical（默认）或horizontal。主机页两栏传`var(--bg)`，Tab栏传`var(--surface)`，主题变化直接跟随CSS变量，不混用底色。

对NScrollbar默认在插槽内查找`.n-scrollbar-container/.n-scrollbar-content`；锁定2.45.3公开实例只有scrollTo/scrollBy，不依赖内部实例ref，该DOM约定与已有主机拖动适配一致，升级库须复核。原生滚动节点传`viewport`，必要时可显式传`content`。外层只需承担布局尺寸；内容增删/标题变化由MutationObserver、viewport/内容尺寸和字体变化由ResizeObserver触发，scroll被动监听并按动画帧合并测量。无溢出、不可见或到达边缘时移除对应端渐隐，小数及弹性滚动容差1px；卸载清理观察器/监听/动画帧。

渐隐用外层伪元素绘制背景色至透明渐变，pointer-events:none，无额外焦点或无障碍节点。固定标题/分组开关、固定标签与新建按钮不包含在渐隐区，滚轮、键盘滚入可见、滚动条及拖动边缘自动滚动沿用实际viewport。useTabScroll只保留顶部纵轮转横向，不重复测量边缘。

```vue
<ScrollFade background="var(--bg)" :size="20">
  <NScrollbar>…</NScrollbar>
</ScrollFade>
```

## 图标来源与替换范围

通用 UI 使用 xicons Vue 3 包 @vicons/ionicons5 0.13.0 与 NIcon；AssetIcon 仅映射语义名称，装饰图标不参与无障碍名称，图标按钮保留名称。

| AssetIcon 名称     | Ionicons 组件          | 位置         |
| ------------------ | ---------------------- | ------------ |
| add                | AddOutline             | 新建标签     |
| sftp               | FolderOpenOutline      | 固定 SFTP    |
| hosts              | ServerOutline          | 主机导航     |
| vault              | LockClosedOutline      | 固定保险箱   |
| credentials        | KeyOutline             | 凭据导航     |
| local-terminal     | HomeOutline            | 本地终端按钮 |
| host-trust         | ShieldCheckmarkOutline | 主机信任导航 |
| connection-history | TimeOutline            | 历史导航     |
| forwarding         | SwapHorizontalOutline  | 转发导航     |

SSH 系统/类型 SVG 为指定资源例外；Serial 使用 HardwareChipOutline、Telnet 使用 GlobeOutline、工作区使用 GridOutline。现有 SFTP 文件类型/分组文件夹自绘图案、系统窗口/品牌资源和库内部图标保留。应用 favicon 为带 Ionicons MIT 声明的 TerminalOutline SVG。

ConnectionTypeDropdown 薄适配 NDropdown/NButton，共用 SSH/Telnet/串口菜单，悬浮显示、bottom-start 对齐，处理外部点击/Tab/Escape/失焦及页面隐藏。NewTabDialog 待退场后再交接所选连接表单，避免焦点陷阱重叠。

## 样式归属

业务 CSS 位于所属 Vue 的 scoped 样式，两个入口共用 src/assets/base.css 的主题 tokens、基础/共享布局、xterm/传送菜单全局适配；字体 CSS 独立。AppTabs 管应用标签，TerminalStage 管分屏/分隔条，TerminalPane 管窗格/终端宿主。标签 class/主题变量使用 app-tab-*，主内容使用 workbench-content，终端区域使用 terminal-area；DOM id、data-app-tab 和关联的焦点/拖动查询一起维护。跨组件动态节点使用明确 :deep() 或入口全局规则，不恢复集中式业务 styles.css。

Vue 使用仓库 Prettier，htmlWhitespaceSensitivity=ignore 仅限 *.vue；pnpm format:vue 自动整理，pnpm check:format:vue 静态校验。依赖精确文本空格的节点使用显式文本表达式或有说明的局部 prettier-ignore，预格式化文本保持原样。

## 生命周期与焦点

- useWorkbenchStore 为标签与终端运行状态源，TerminalRegistry 按稳定 terminalId 持有 xterm。排序/显隐/分屏/交换/重连保留实例，固定页不创建终端。
- AppTabs 使用 tablist/roving tabindex、方向键/Home/End/Enter/空格、关闭/菜单键；库主题管理颜色和圆角，分屏几何由业务控制。
- 主机页显隐挂原生 DOM，分组/列表独立 NScrollbar；设置正文滚动、自动应用/重试 footer 固定，配色网格自然展开，编辑默认折叠。
- AppDialog 转发 after-enter/after-leave，连接/认证退场后恢复当前活动终端焦点；模态和录制优先。弹层标记随实际 open 状态清理，避免隐藏节点拦截快捷键。
- 搜索使用 NSelect remote/filterable/header 与 focusInput；字段复选框鼠标勾选保留输入光标，空查询隐藏菜单。全局 NewTabDialog 直接列结果并复用搜索配置。
- 装饰文字不可选；输入、可复制正文及 xterm 保留选择能力，不添加全局鼠标/选区拦截。

## 安全与认证

VaultGate 使用库表单完成首次密码或自动/密码解锁，缓存失败保留已解锁并显示警告。业务未解锁不挂载，窗口操作保留。秘密不进入页面长期状态；私钥/文件正文 Rust 读取，不开放任意本地路径读写。库适配不放宽 CSP script-src、导航或 IPC owner。

SSH 主机/快速连接提供密码和公钥，OTP/PAM 由服务端协商；Telnet 隐藏 SSH 认证并显示明文说明，Serial 仅参数配置。设置窗口只允许设置、专用保险库改密/JSON 导出及备份同步命令；改密采用独立共享 model 的 NForm 与 AppDialog footer，不写入设置草稿。详见 [安全设计](08-security.md)。

## 备份界面与模块复用

`modules/backup/BackupPanel.vue` 复用 Naive UI 的配置表单（含备份密码）、服务与操作选择、版本列表和预览确认框，供独立设置与首次 VaultGate 初始化共同装配；密码与确认字段绑定同一配置表单的共享响应式model，导入确认弹窗独立，避免嵌套 HTML form。`useCloudBackup.ts` 管理操作、分页元数据、脱敏配置与短期预览令牌；`useBackupRestored.ts` 订阅无正文的失效事件。未来新增云提供方复用 desktop 类型化门面及 Rust provider 边界，不在组件中实现网络或加密。

云配置显式保存，上传/列表前也会保存有效配置；不并入 AppSettings 自动保存草稿。秘密只作为用户输入短期持有，已保存秘密仅显示 hasSecret/hasBackupPassword 占位；卸载清空密码输入，成功提交清除相应秘密。备份密码与Token一样在配置时加密保存，后续上传/下载不重复输入；初次或修改密码须确认，旧配置补填后升级为版本2。下载元数据使用配置中的密码解密，按10项分页；正文不进入 WebView/Pinia。导入预览使用 AppDialog 独立 footer，确认前不写入，忙碌时禁用操作并阻止设置关闭；配置或备份密码变化清空旧列表/预览。首次云端初始化仅 setup 状态展示，现有 locked 库保留普通解锁。详见[桌面交互](09-desktop-ux.md#13-手动备份与云端同步)和[备份设计](22-data-backup.md)。

## 表单响应式模型与校验时序

沿用 [Naive UI Form](https://www.naiveui.com/zh-CN/os-theme/components/form) 的 model/rules/path/v-model：NForm 和控件共享同一响应式字段，提交前 validate，必填星号由 required 规则生成。禁止内联对象、对象展开或 computed 快照作为校验 model；已有 ref 可用 reactive 保留字段引用。嵌套对象的规则与 path 同步，不能用 nextTick/盲目清错掩盖旧值。

NSelect 更新回调后会立即触发校验，FormItem 按当前 model/path 读取；系统文件导入、颜色重置等程序更新后通过公开 FormItem.validate 重验相应字段。普通 input/change/blur 仍保留，快速连接名称、可选秘密和服务器允许空值的回答不强制必填。

主机编辑使用 NGrid/NFormItemGi 双列及 NGi 跨列，不叠加自定义 grid 行距。配色与常规表单分离，禁止嵌套 HTML form。结构检查检查常见 model 快照误用，实际选中/清空/纠错、鼠标/键盘和重开仍需运行验收。

## 用户指定的SSH系统/类型图标例外

28个指定系统/类型 SVG 位于 src/assets/svg/systems，语义 ID、原文件名、SHA-256 见 [清单](../resources/host-system-icons.json)。SSH 默认项使用 host.react-ecf6b335.svg；不接收运行时外部 SVG。

HostSystemBadge 以 NAvatar 承载系统色底和白色 Logo，HostSystemIcon 使用静态白名单并逐实例隔离 SVG 内部 ID/引用。卡片/列表/侧栏/选择器共用资源，默认插槽居中并保留库内联缩放。手动非默认图标优先于自动识别；来源/许可仍需发布前核验。详见 [资源说明](../resources/README.md)。

## 依赖与验证

固定包要求 Node >=20、Vue peer ^3.0.0，MIT；来源：[官网](https://www.naiveui.com/)、[仓库](https://github.com/tusen-ai/naive-ui)、[固定版本元数据](https://registry.npmjs.org/naive-ui/2.45.3)。API 以锁定包源码为准，不根据 sideEffects 声明推断实际打包体积。

待验收包括三平台高 DPI/主题、表单与弹层/IME、横纵标签、隐藏终端、文件授权/拖放、字体/配色和跨窗口同步。类型、结构、格式和资源摘要仅静态检查；验证状态集中见 [产品需求](01-product-requirements.md#9-已有验证与证据边界)。

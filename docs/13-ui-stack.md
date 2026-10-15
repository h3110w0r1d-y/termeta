# UI 技术选型与 Reka UI 桌面适配评估

核查日期：2026-10-15。结论：**适合采用 Reka UI 作为基础交互控件，配合项目组件封装与自定义样式建设桌面 UI**。已接入 Reka UI 2.11.0 并封装 Dialog/确认与输入服务，完成前端静态类型检查，未运行三平台验证。

## 1. 当前状态与目标方案

前期模板现已替换为模块化工作区，已接入 Reka Dialog 及项目封装；其余 primitive 按需求选用。前一版文档将 Reka 视为可选候选；本次根据用户“高度自定义、适合桌面应用”的要求，将方向明确为：

```text
Vue 3 + TypeScript
  ├── Reka UI primitives        基础交互、ARIA、焦点/键盘行为
  ├── 项目 ui 封装              统一控件 API、状态、样式与默认值
  ├── CSS tokens + 项目样式     主题、紧凑密度、尺寸、层级、视觉
  ├── 工作区 / TerminalRegistry 稳定 Tab 与 xterm 生命周期
  └── Tauri 原生能力            系统菜单、文件选择、窗口集成
```

Tailwind 可以作为团队选择的样式书写工具，但不是采用 Reka 的前提；首期默认沿用 Vue scoped CSS + 公共 CSS 变量，避免同时引入多套样式体系。该组合保留完整视觉控制，同时减少重复实现复杂基础交互的工作。

## 2. 官方能力核查

| 需求 | 官方依据 | 适配判断 |
| --- | --- | --- |
| 自定义外观 | 无预设视觉样式，可用普通 CSS 与状态 data attributes 定制 | 适合终端客户端的紧凑界面和独立品牌主题 |
| 结构组合 | `asChild` 可把行为组合到项目元素/组件；封装需正确透传 props/events/refs | 可封装自己的控件外观；不是任意改变语义仍自动可访问 |
| 键盘/焦点 | 组件按 WAI-ARIA 模式处理相关行为 | 为桌面键盘操作提供基础，仍要与 xterm 按键协调 |
| 状态控制 | 支持受控模式 | 主机选择、对话框和标签状态由应用 stores 驱动 |
| 菜单与弹层 | Dialog、Context Menu、Dropdown Menu、Tooltip 等 | 覆盖主机右键、认证、设置与操作提示 |
| Tabs | 有受控切换、手动/自动激活、键盘导航 | 可用作标签语义基础，终端宿主必须单独保活 |
| Splitter | 水平/垂直、嵌套、键盘调节等 | 可用于侧栏尺寸与未来分屏，不等于完整 Dock 布局 |
| Tree | 当前官方页面明确标 Alpha | 分组树候选，隔离适配并单独验证，不能按稳定控件承诺 |

来源：[介绍](https://reka-ui.com/docs/overview/introduction)、[样式](https://reka-ui.com/docs/guides/styling)、[组合](https://reka-ui.com/docs/guides/composition)、[Dialog](https://reka-ui.com/docs/components/dialog)、[Context Menu](https://reka-ui.com/docs/components/context-menu)、[Tabs](https://reka-ui.com/docs/components/tabs)、[Splitter](https://reka-ui.com/docs/components/splitter)、[Tree](https://reka-ui.com/docs/components/tree)。

这些证明的是库的网页组件能力；“适合本项目桌面 UI”是结合需求作出的工程判断，不是官方对 Tauri 三种 WebView 的兼容认证。文档当前形态与具体稳定 release 可能存在差异，落地时核对同一 tag 的 API/peerDependencies/许可证与变更记录。[维护者仓库](https://github.com/unovue/reka-ui)

## 3. 项目组件映射

| 产品部位 | 封装方向 | 生命周期与限制 |
| --- | --- | --- |
| 密码/指纹/主机编辑 | AppDialog / AppAlertDialog | 焦点恢复到发起控件；拒绝为安全默认，后台 challenge 不突然抢焦点 |
| 主机/标签右键 | AppContextMenu | 无远端命令自动执行；明确断开与关闭 Tab |
| 操作菜单 | AppDropdownMenu | 统一禁用态、图标槽、快捷键展示，不在全局截获终端输入 |
| 主题/Shell/认证方式 | AppSelect / AppCombobox | 受控值、可搜索项按需使用；不把秘密放进选项标签 |
| 设置开关/说明 | AppSwitch / AppCheckbox / AppTooltip | 提供文本标签与键盘访问，tooltip 不承载唯一必要说明 |
| 字体/表单文本 | AppInput / AppButton 等原生元素封装 | 简单控件可直接封装原生元素，无须每个元素都套 primitive |
| 主机分组树 | HostTree 业务适配层 | 优先评估 Tree；Alpha 不达标时采用明确支持键盘的项目树实现 |
| 标签栏 | SessionTabStrip | 可复用 Reka Tabs 交互，但关闭按钮不能嵌套进另一个 button；拖拽排序另做 |
| 终端内容区 | TerminalHost / TerminalRegistry | 由稳定 terminalId 管理；切换/失败/重连不得卸载或重置 |
| 侧栏宽度/未来分屏 | WorkspaceSplitter | 受控尺寸，布局持久化仍通过项目设置/布局模型 |
| SFTP 表格/双栏/传输队列 | FileTable / FileBrowser / TransferQueue | 表格虚拟化、文件操作/进度与拖拽由业务实现，不声称 Reka 已提供完整文件管理器 |
| 快捷键编辑 | KeyBindingRecorder / PointerBindingEditor | 可复用 Dialog/Input；命令路由、冲突与修饰键点击不是 primitives 自动提供 |
| 终端配色编辑 | TerminalSchemeEditor / ColorPicker | 项目自定义取色/数值输入/预览，可复用 Popover/Slider；不强依赖 Alpha 控件 |
| SFTP 停靠与原生拖出 | WorkspaceSplitter + 平台 drag 适配器 | Splitter 仅负责尺寸，不实现原生文件拖出；三位置切换不卸载 xterm |
| macOS 系统菜单/文件选择 | Tauri 原生菜单/对话框 | 网页 Menubar 不替代系统级菜单与键盘约定 |

只在当前模块需要时引入对应 primitive，不一次性搭建全部封装。Reka 为单包分发，但具体 tree-shaking 效果要用实际构建验证，本轮不报告包体积或性能收益。

## 4. 样式与桌面视觉基线

公共 tokens 放在 `src/app/styles/`（目标目录），统一背景/面板/边框/文字/选中/状态色、间距、控件高度、圆角、阴影、字号和焦点环。初始设计密度可用 28–32 px 常用控件高、12–13 px 界面字体、可放大的点击区域；这是设计起点，需真实高 DPI/缩放和可访问性验证，不固定成不可调整的像素约束。

使用 `[data-state]` 等公开状态属性和项目 class 定制，不依赖控件内部深层 DOM 选择器。应用 CSS tokens 与终端方案分别管理，TerminalThemeService 转换独立 scheme 为 xterm theme；允许显式配置跟随明暗，不能强制绑定。应用字号与终端字号分别设置。颜色之外提供图标/文本状态，避免“只有一个红点”表达失败。

Portal 到根节点的菜单/对话框统一 overlay-root 与 z-index tokens，确保继承主题、缩放、字体和可访问标签；注意 Vue scoped 样式与 Teleport 的作用范围，公共弹层样式放在正确作用域。不要把“无样式”理解为没有定位/交互所需的功能样式。

## 5. 终端与基础控件的集成规则

1. **稳定宿主**：Registry 按 terminalId 持有 xterm，Tab/连接失败/重连均不改变该 key。若采用 Reka Tabs，按选定版本使用可靠的保留挂载方案或把持久终端宿主独立管理；不能照抄演示默认 Content 生命周期。
2. **保留本地交互**：离线仅禁止向旧连接写入，滚动、选择、复制和搜索保持可用；不能整体禁用终端 DOM。失败提示不能覆盖终端成不可操作页面。
3. **焦点所有权**：菜单/弹窗打开时暂停将其键入发送远端；关闭时恢复原 pane 的焦点。IME composition、Escape、Enter、Tab、箭头按当前焦点区域路由，不用全局捕获一刀切。
4. **受控状态**：连接状态来自后端快照，激活标签来自 WorkspaceStore；不要让 primitive 的内部状态成为第二套工作区真相。
5. **布局尺寸**：Splitter/窗口尺寸变化触发可见 xterm 的 fit/resize；隐藏或折叠时不发送 0×0。拖动不销毁终端或重复建连接。
6. **原生操作分工**：应用全局菜单用 Tauri，局部右键菜单可用 Reka；两者共享业务命令注册，避免快捷键执行两次。
7. **安全配置**：浮层定位/功能样式可能涉及动态 style，需与发布 CSP 联合验证。不能为 UI 快速可用就关闭 CSP；也不能未测就声称严格 style-src 下所有组件可用。

## 6. 与其他路径的取舍

| 路径 | 对当前需求的适配 | 本项目选择 |
| --- | --- | --- |
| Reka primitives + 自有设计系统 | 外观、密度和组合自由；基础交互可复用；需要设计与封装工作 | 采用方向 |
| 整套预设风格组件库 | 通常可更快搭标准表单；独立桌面视觉需要进一步定制 | 本轮不引入第二套完整库；未做逐库 API 对比 |
| 全部从原生 HTML 自建 | 自定义空间大；焦点、键盘、ARIA 和弹层等工作量高 | 简单控件适用，复杂通用交互优先复用 Reka |
| 基于 Reka 的现成组件模板 | 可参考设计/封装，但另有样式与代码维护决策 | 不作为强制依赖；采用前另核查许可与版本 |

没有证据说明 Reka 必然比其他方案更轻或更快，也不把“桌面风格”归功于库默认主题；桌面观感来自设计 tokens、布局、密度、焦点和系统行为的共同实现。

## 7. 采用条件与验证清单

- 选择稳定 release，核对当前 Vue/TypeScript/Vite 版本与 peerDependencies，锁定包并记入预研；不混用 Radix Vue 旧教程和 Reka 新版 API。
- 优先集成 Dialog、Context Menu、Select、Tooltip 等当前需要的封装；Tree Alpha 与复杂布局单独通过适配层隔离。
- SP-07 验证三平台 WebView：原终端跨失败/重连保活、上下文菜单焦点、IME/快捷键冲突、Portal 主题、缩放、CSP 与 Splitter resize。
- 局部组件不能满足需求时替换该适配器，不要求整个终端/存储架构跟着更换；具体未通过项保留待验证状态。

当前结论可用于后续开发基线。依赖可按开发任务需要主动安装，仍不得主动构建/运行项目；本轮已安装并静态核对，未运行集成。进度与执行约束见 [路线](10-roadmap.md) 和 [AGENTS.md](../AGENTS.md)。

## 8. 新增交互专项边界

[快捷键](14-shortcuts.md) 中的 CommandRegistry/KeymapService/ShortcutRouter 由应用拥有，Reka 焦点与键盘行为需接入统一优先级。[配色编辑](15-terminal-themes.md) 的预览、取色、导入和持久化属于项目业务；内置颜色控件不是选用 Reka 的必要前提。

[SFTP](16-sftp-workspace.md) 两种布局复用同一 FileBrowser/TransferQueue，左右/底部停靠可用 Splitter 受控尺寸实现。文件列表虚拟化、内部拖拽、OS drop/drag-out 各自验证，不能因存在 Splitter/Tree 就承诺完整桌面文件管理能力。

## 首期实际集成（2026-10-15）

Reka UI 2.11.0 用于 AppDialog、共享确认/输入和粘贴预览；简单控件使用原生元素。SettingsDialog拆为ThemeSettings、ShortcutSettings、SecuritySettings，配色控件独立ColorField。终端生命周期由TerminalRegistry管理，不依赖Dialog/Tabs显隐。CSS tokens位于src/styles.css。vue-tsc/Prettier静态通过，CSP/Portal/焦点/IME/SP-07~09未运行验证。

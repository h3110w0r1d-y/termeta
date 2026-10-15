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

## 生命周期与焦点

- WorkspaceStore是唯一标签状态，固定首页/SFTP不创建session。TerminalPane按terminalId保活，显隐、排序、分屏和重连不重建或清空xterm。
- WorkspaceTabs采用单一tablist、roving tabindex、方向键/Home/End/Enter/空格、Delete关闭及菜单键；固定页与会话滚动区分离。标签主题色和圆角读取useThemeVars，拖动与分屏几何见 [工作区](18-terminal-workspaces.md)。
- 首页主机/凭据各有NScrollbar，设置占满剩余空间且footer独立；主机页v-show挂原生容器，避免滚动组件观察器根元素破坏显隐。三页共享NH1/NText页头。
- 配色网格自然展开，随设置正文滚动；自定义编辑默认折叠，展开后才创建预览。字体下拉固定12项，选项及当前值以对应本地字体显示，资源见 [字体清单](../public/fonts/README.md)。
- 活动模态、选择/取色浮层优先处理焦点与Escape；关闭动画保留DOM时，AppDialog活动标记已经移除，避免确认后的粘贴被自身动画拦截。
- 无关界面文字默认不可选；输入、可复制正文和xterm保留选择能力，不添加全局选择/鼠标拦截。

## 安全与认证

主机/快速连接提供密码和公钥认证；OTP/PAM/Keyboard-interactive由服务端协商动态触发。凭据仅密码/私钥，私钥可带解密密码；主机专属密码非空自动保存、显式空值改为连接时询问，OTP不保存。认证挑战不能因浮层Escape或焦点变化被意外提交/取消。

私钥及文件正文字节在Rust读取；前端不获得任意本地路径读写能力。保留CSP既定style-src范围，不为组件库放宽script-src、导航或IPC所有权检查。SFTP拖放与限制见 [SFTP设计](16-sftp-workspace.md)。

## 依赖与验证

已核对2.45.3包manifest、类型和许可证：Node要求>=20，Vue peer为^3.0.0，MIT。包声明sideEffects:false，未构建因此不报告实际体积或tree-shaking效果。来源：[官网](https://www.naiveui.com/)、[仓库](https://github.com/tusen-ai/naive-ui)、[固定版本元数据](https://registry.npmjs.org/naive-ui/2.45.3)。API以锁定包源码为准。

待验收：三平台高DPI、系统主题、弹层焦点/IME、表单Enter/校验、横纵标签、隐藏终端、文件选择/拖放和颜色往返。类型、格式及静态契约检查不代表桌面交互已通过，见 [验证方案](11-validation.md)。

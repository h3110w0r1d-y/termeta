# 静态资源来源

`terminal-palettes.json` 包含从用户提供的 Termius 解包目录静态提取的64套终端配色，保留原名称与全部终端色值。前后端使用同一 JSON：前端映射为 Scheme，Rust 仅从中读取内置 ID 名单，避免维护两份白名单。默认方案为 Pro。

来源文件为 `ui-process/assets/reconnectSaga-871ae692.js`，另外三个进程包中的同一数组已做逐字节一致性核对。只截取21,621字节数组进行 TypeScript AST 数据解析，不运行解包程序，也不输出整行压缩 JS。来源文件与数组 SHA-256 见 [提取记录](terminal-palettes.provenance.json)。

映射：`colorPaletteOverrides` 的16色依原顺序保存为 ANSI；`backgroundColor` / `foregroundColor` / `cursorColor` / `selectionColor` 分别映射背景、前景、光标与选区。源数据未定义光标文字反色和失焦选区，分别沿用背景色与原选区色。`app.terminalUIColor` 不进入终端 Scheme。

提取工具为 [extract-terminal-palettes.mjs](../scripts/extract-terminal-palettes.mjs)，接收输入 JS 路径与输出 JSON 路径；仅接受对象/数组/字符串字面量，拒绝表达式、方法与函数调用。脚本打印摘要，不打印完整源码。提取工具依赖仓库已有 TypeScript，不增加依赖。

读取旧预设引用时，暗色回退Nord Dark、浅色回退Nord Light；同名Ocean采用当前提取值。归一结果在主动保存时持久化，不删除自定义方案。终端通过现有xterm主题接口热应用，行为与限制见 [配色说明](../docs/15-terminal-themes.md)。

## 主机搜索拼音表

按用户指定，从本地 roco-helper-web 项目的 `src/assets/conf/PINYIN_CONF.ts` 复制20903条Unicode码点到拼音候选的配置至 [PINYIN_CONF.ts](../src/assets/conf/PINYIN_CONF.ts)，仅将文件头注释改为中文，配置数据一致。匹配逻辑参考该项目 `src/modules/big-map/petNameSearch.ts`，适配为 [主机搜索工具](../src/modules/hosts/search.ts)，支持原文、全拼、首字母、多音字、部分音节和中英/拼音混合输入；不引入上游业务数据或新依赖。复制日期与原始/本地摘要见[来源记录](pinyin.provenance.json)。

## 界面图标

新建标签、主机、保险箱、凭据、本地终端、主机信任、连接历史和固定 SFTP 标签统一使用 [xicons](https://xicons.org/) 的 `@vicons/ionicons5`，通过 Naive UI `NIcon` 渲染。`AssetIcon.vue` 仅保留语义名称到库组件的静态映射，原先对应的 8 个 Termius 界面 SVG 已移除。主机系统/类型图标继续保留下述原始素材，不随界面图标替换。

## SSH 系统图标与检测命令

按用户指定，原样复制28个系统/类型SVG到[src/assets/svg/systems](../src/assets/svg/systems)，使用语义文件名。映射参考解包reconnectSaga-871ae692.js，原文件名与SHA-256见[host-system-icons.json](host-system-icons.json)；raspbian与pi复用pi素材，suse使用原suze素材；SSH默认项按用户指定改用`host.react-ecf6b335.svg`，原样保存为`systems/ssh.svg`（保留id=ssh及原背景色）。素材复用仅按本次用户指定范围，不声明这些素材为项目自有或另行开源许可。

[ssh-os-detection.json](ssh-os-detection.json)为重新编写的5份固定只读命令：Shell探测、POSIX、Fish、RouterOS及Windows，参考Termius检测流程，POSIX/Fish优先标准os-release。Rust通过include_str读取、设置页直接导入同份JSON，设置不接受脚本编辑。命令不会作为本地脚本执行。系统类型仅用于图标展示，细节见[SSH设计](../docs/05-ssh-design.md#10-一次性主机系统识别)。

运行`node scripts/check-system-resources.mjs`只做资源摘要、主动内容、图标白名单及命令共用入口的静态检查，不编译、不启动项目、不执行检测命令。

主机卡片颜色同样来自用户指定解包数据：`reconnectSaga-e37b5571.css`中的`--system-*`变量，通用SSH回退`--dark-blue-solid`；映射后的`colorVariable`和`background`保存在图标清单。Debian源值为`#CE0056`，不将截图取色`#BD2757`视为源码常量。`reconnectSaga-871ae692.js`的`ShapedIcon/useStyles$3K`定义40×40px、12px圆角、白色图标和彩色背景，没有单独描边；项目通过NAvatar复用该尺寸与色值。

配色缩略图按用户指定解包文件 `ui-process/assets/reconnectSaga-871ae692.js` 中 `ThemeItem`、`mapSchemeToThemeData` 与 `useStyles$3o` 的DOM结构和尺寸适配，源码为 `src/modules/settings/SchemeThumbnail.vue`。仅使用现有Scheme色值，不引入React运行时、使用人数或品牌图标；六色条按用户要求始终显示。

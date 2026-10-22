# 静态资源来源

本文记录资源来源、许可边界与复现入口，行为契约见 [桌面交互](../docs/09-desktop-ux.md) 和各领域文档。来源或摘要核对不等于发布许可与平台展示验收通过。

## 桌面应用品牌图标

当前默认采用用户选定的 **8-dark** 品牌图稿（三层正交终端面板与柔和扩散阴影），提供 [深色底图稿](brand/termeta-icon.svg)、[亮色底图稿](brand/termeta-icon-light.svg) 和共用的 [透明标志](brand/termeta-symbol.svg)，来源及 SHA-256 见 [品牌来源](brand/provenance.json)。第一层底面按正面描边外沿等距扩大，厚度采用扩大后的前后轮廓与公共切线组成的单一连续包络，在左上、右下圆角外沿切点连接。亮色版只调整底板配色，三层面板共用相同形体、渐变与阴影。

[生成脚本](../scripts/generate-app-icons.py) 只调整终版图稿的画布取景，不改面板、配色或阴影。通用版去除底板外留白，用于 PNG、ICO 和 [网页图标](../public/terminal.svg)；macOS 专用版将原图稿等比居中，底板宽度占画布的 824/1024，保留原底板长宽比与平台留白，不用通用 ICNS 覆盖。

Python 工具环境需 Python ≥3.10，Pillow 12.3.0 与 resvg-py 0.5.0 固定在 [工具依赖](../scripts/icon-requirements.txt)；只用于资源生成，不加入应用依赖。桌面 PNG/ICNS 由仓库锁定的 Tauri CLI 2.12.3 生成，ICO 各尺寸由矢量直接渲染并封装，不纳入移动端输出：

```sh
python scripts/generate-app-icons.py
```

可选 `--artifacts-dir <目录>` 保留平台母图、图标副本、预览及摘要报告。脚本执行 `tauri icon`，不触发项目编译、启动或打包。Windows ICO 显式按尺寸降序排列，首项必须是 256×256、32 位高清图，并校验每项可解码；ICNS 包含 1024×1024。静态资源校验不代表系统图标缓存、Dock/任务栏、高 DPI 或安装包展示验收通过。

默认同时生成深浅两套，`--theme dark|light|all` 可限定输出。深色版应用于 `src-tauri/icons/` 与 `public/terminal.svg`；亮色备选套装位于 `src-tauri/icons/light/` 与 `public/terminal-light.svg`，不自动切换应用图标。需要修改正面轮廓时，先运行 [厚度修整脚本](../scripts/refine-brand-icons.py)，它读取正面描边宽度、外扩底面圆角并求公共切线，同步生成亮色底 SVG，再生成平台资源。

## 终端配色

[terminal-palettes.json](terminal-palettes.json) 为用户提供的 Termius 解包数据中提取的 64 套方案，保留原名称与颜色，默认 Pro。前端主题与 Rust 内置 ID 白名单共用此 JSON。

来源为 `ui-process/assets/reconnectSaga-871ae692.js`，文件与数据数组 SHA-256 见 [来源记录](terminal-palettes.provenance.json)。[提取工具](../scripts/extract-terminal-palettes.mjs) 仅用 TypeScript AST 接受对象/数组/字符串字面量，不运行解包程序或接受表达式。

源 16 色映射 ANSI，backgroundColor/foregroundColor/cursorColor/selectionColor 映射对应主题字段，app.terminalUIColor 映射可选的 colors.ui 标签栏辅助色；缺少的光标文字色沿用背景色，失焦选区沿用选区色。旧暗/浅预设分别归一 Nord Dark/Nord Light，Ocean 使用当前提取值；不删除自定义方案。主题与 JSON 行为见 [配色约束](../docs/09-desktop-ux.md#6-应用外观与终端配色)。

## 主机搜索拼音表

按用户指定，从 roco-helper-web 的 `src/assets/conf/PINYIN_CONF.ts` 复制 20903 条 Unicode 码点配置到 [PINYIN_CONF.ts](../src/assets/conf/PINYIN_CONF.ts)，仅文件头注释改为中文。匹配逻辑参考其 `petNameSearch.ts`，适配在 [search.ts](../src/modules/hosts/search.ts)，不引入上游业务数据或新依赖。日期与摘要见 [来源记录](pinyin.provenance.json)。

## 界面及 SSH 系统图标

通用界面图标采用 [Tabler Icons](https://tabler.io/icons) 的 `@tabler/icons-vue` 3.49.0，MIT 许可；[AssetIcon](../src/components/ui/AssetIcon.vue) 只做语义映射。SFTP 文件类型与主机分组文件夹同样使用 Tabler 组件。

用户指定的 28 个 SSH 系统/类型 SVG 原样复制到 [systems](../src/assets/svg/systems)。原文件名、语义映射、颜色和 SHA-256 见 [host-system-icons.json](host-system-icons.json)：raspbian/pi 复用 pi，suse 对应 suze，默认 ssh 使用用户指定 host 素材。复用仅限用户指定范围，不声明项目拥有这些素材或另行授予开源许可。

[ssh-os-detection.json](ssh-os-detection.json) 包含重新编写的 5 份固定只读检测命令，覆盖 Shell、POSIX、Fish、RouterOS 和 Windows。Rust 与设置页共用，不能编辑或在本机执行；系统类型仅作图标信息，见 [SSH](../docs/05-ssh-design.md#10-一次性主机系统识别)。

`node scripts/check-system-resources.mjs` 只静态检查摘要、主动内容、图标白名单和命令共用入口，不构建、启动或执行检测命令。

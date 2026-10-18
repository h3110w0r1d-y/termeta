# 静态资源来源

本文记录资源来源、许可边界与复现入口，行为契约见 [桌面交互](../docs/09-desktop-ux.md) 和各领域文档。来源或摘要核对不等于发布许可与平台展示验收通过。

## 桌面应用品牌图标

`src-tauri/icons/` 来自用户确认的 Termeta 标志母图，由锁定 Tauri CLI 2.12.3 生成。通用母图用于 PNG/ICO，macOS 专用母图用于 ICNS；后者按 [Apple 资源](https://developer.apple.com/design/resources/)保留平台留白，不能用通用 ICNS 覆盖。

母图与可编辑 SVG 位于用户外部工作目录。重新生成后，将通用 PNG/ICO 与专用 ICNS 合并到 `src-tauri/icons/`，不纳入移动端输出：

```sh
pnpm tauri icon "$ICON_STANDARD_SOURCE" --output "$ICON_STANDARD_OUTPUT"
pnpm tauri icon "$ICON_MACOS_SOURCE" --output "$ICON_MACOS_OUTPUT"
```

## 终端配色

[terminal-palettes.json](terminal-palettes.json) 为用户提供的 Termius 解包数据中提取的 64 套方案，保留原名称与颜色，默认 Pro。前端主题与 Rust 内置 ID 白名单共用此 JSON。

来源为 `ui-process/assets/reconnectSaga-871ae692.js`，文件与数据数组 SHA-256 见 [来源记录](terminal-palettes.provenance.json)。[提取工具](../scripts/extract-terminal-palettes.mjs) 仅用 TypeScript AST 接受对象/数组/字符串字面量，不运行解包程序或接受表达式。

源 16 色映射 ANSI，backgroundColor/foregroundColor/cursorColor/selectionColor 映射对应主题字段；缺少的光标文字色沿用背景色，失焦选区沿用选区色。旧暗/浅预设分别归一 Nord Dark/Nord Light，Ocean 使用当前提取值；不删除自定义方案。主题与 JSON 行为见 [配色约束](../docs/09-desktop-ux.md#6-应用外观与终端配色)。

## 主机搜索拼音表

按用户指定，从 roco-helper-web 的 `src/assets/conf/PINYIN_CONF.ts` 复制 20903 条 Unicode 码点配置到 [PINYIN_CONF.ts](../src/assets/conf/PINYIN_CONF.ts)，仅文件头注释改为中文。匹配逻辑参考其 `petNameSearch.ts`，适配在 [search.ts](../src/modules/hosts/search.ts)，不引入上游业务数据或新依赖。日期与摘要见 [来源记录](pinyin.provenance.json)。

## 界面及 SSH 系统图标

通用界面图标采用 [Tabler Icons](https://tabler.io/icons) 的 `@tabler/icons-vue` 3.49.0，MIT 许可；[AssetIcon](../src/components/ui/AssetIcon.vue) 只做语义映射。SFTP 文件类型与主机分组文件夹同样使用 Tabler 组件。

用户指定的 28 个 SSH 系统/类型 SVG 原样复制到 [systems](../src/assets/svg/systems)。原文件名、语义映射、颜色和 SHA-256 见 [host-system-icons.json](host-system-icons.json)：raspbian/pi 复用 pi，suse 对应 suze，默认 ssh 使用用户指定 host 素材。复用仅限用户指定范围，不声明项目拥有这些素材或另行授予开源许可。

[ssh-os-detection.json](ssh-os-detection.json) 包含重新编写的 5 份固定只读检测命令，覆盖 Shell、POSIX、Fish、RouterOS 和 Windows。Rust 与设置页共用，不能编辑或在本机执行；系统类型仅作图标信息，见 [SSH](../docs/05-ssh-design.md#10-一次性主机系统识别)。

`node scripts/check-system-resources.mjs` 只静态检查摘要、主动内容、图标白名单和命令共用入口，不构建、启动或执行检测命令。

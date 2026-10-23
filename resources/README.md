# 静态资源与来源

本目录保存应用和工具使用的共享资源，以及分发时需要保留的来源、版权和许可信息。资源说明不包含开发机绝对路径；第三方来源记录不等于已取得再分发授权。

## 目录用途

| 文件或目录 | 用途 |
| --- | --- |
| [brand/](brand/) | 品牌 SVG 源图，供图标生成工具使用 |
| [terminal-palettes.json](terminal-palettes.json) | 前端配色与 Rust 内置方案 ID 白名单的共同数据源 |
| [host-system-icons.json](host-system-icons.json) | SSH 系统图标映射、样式和素材摘要 |
| [ssh-os-detection.json](ssh-os-detection.json) | Rust 和设置界面共用的固定只读检测命令 |
| [fonts/](fonts/) | 内置字体的来源、版本、许可和文件摘要；字体正文及许可证位于 [public/fonts/](../public/fonts/) |

## 桌面应用品牌图标

品牌图稿由项目维护者提供并确认，包含 [深色底图稿](brand/termeta-icon.svg)、[亮色底图稿](brand/termeta-icon-light.svg) 和 [透明标志](brand/termeta-symbol.svg)。深色为当前默认，亮色为备选；来源说明不替代品牌资源的发布许可确认。

[生成脚本](../scripts/generate-app-icons.py) 从 SVG 生成桌面 PNG、ICO、ICNS 和网页图标，只调整画布取景，不改变图稿内部形体。通用版去除外部留白，macOS 版保留平台留白。默认生成深浅两套资源：深色输出到 `src-tauri/icons/` 和 `public/terminal.svg`，亮色输出到 `src-tauri/icons/light/` 和 `public/terminal-light.svg`；应用图标不随界面主题自动切换。

生成工具要求 Python ≥3.10，Pillow 12.3.0 和 resvg-py 0.5.0 固定于 [工具依赖](../scripts/icon-requirements.txt)，并使用仓库锁定的 Tauri CLI 2.12.3：

```sh
python scripts/generate-app-icons.py
```

支持 `--theme dark|light|all`，可用 `--artifacts-dir <目录>` 保存预览和摘要报告。该脚本调用 `tauri icon`，不编译、启动或打包应用。[轮廓修整工具](../scripts/refine-brand-icons.py) 用于同步调整源图厚度与亮色版本。平台图标展示、高 DPI 和缓存行为仍需实际验收。

## 终端配色

[terminal-palettes.json](terminal-palettes.json) 包含从维护者提供的 Termius 解包数据中静态提取的 64 套方案，保留名称与颜色，默认 Pro。[提取工具](../scripts/extract-terminal-palettes.mjs) 只解析对象、数组和字符串字面量，不执行源程序。

来源入口为 `ui-process/assets/reconnectSaga-871ae692.js`。原文件 SHA-256 为 `81f4ccca739dc8fd469d24a3b30ec233958e5b321a79ab35b71c9b4bf89ea867`，配色数组 SHA-256 为 `307748f9b6a431d89d9629ef9454ffb26b250c0215d9b09eb49959e845bef584`。

16 色数组映射为 ANSI，背景、前景、光标、选区和 UI 辅助色映射为对应主题字段；缺少的光标文字色沿用背景，失焦选区沿用选区色。应用行为见 [配色约束](../docs/09-desktop-ux.md#6-应用外观与终端配色)。

**该配色集合尚未确认独立再分发授权，不声明采用 MIT、OFL 或其他开源许可证。**

## 主机搜索拼音表

[PINYIN_CONF.ts](../src/assets/conf/PINYIN_CONF.ts) 的 20903 条码点配置来自 roco-helper-web 的 `src/assets/conf/PINYIN_CONF.ts`，仅文件头注释改为中文；[主机搜索](../src/modules/hosts/search.ts) 参考其 `src/modules/big-map/petNameSearch.ts` 适配，不引入上游业务数据或网络请求。

原表 SHA-256 为 `04c964c89347b15ae152786a957351e88dcf3818394f4a27b00ad2866beccee5`，仓库表 SHA-256 为 `b8a1bfbcb4021670e611cf97bc0adb31108e553443d22a7d64fe8b97190e583c`。上游许可及再分发授权仍需在发布前确认。

## 界面及 SSH 系统图标

通用界面图标使用 MIT 许可的 [Tabler Icons](https://tabler.io/icons)，依赖固定为 `@tabler/icons-vue` 3.49.0。

28 个 SSH 系统/类型 SVG 来自维护者指定的 Termius 图标素材，保留原始字节，位于 [systems/](../src/assets/svg/systems/)。名称映射、颜色和 SHA-256 见 [host-system-icons.json](host-system-icons.json)。不声明项目拥有这些素材，也不额外授予开源许可；再分发授权仍待确认。

[ssh-os-detection.json](ssh-os-detection.json) 包含独立编写的 Shell、POSIX、Fish、RouterOS 和 Windows 固定只读命令，仅用于已认证 SSH 会话的系统识别，不在本机执行。行为见 [SSH 设计](../docs/05-ssh-design.md#10-一次性主机系统识别)。

`node scripts/check-system-resources.mjs` 仅静态检查素材摘要、主动内容、图标白名单及检测命令的共用入口，不运行远端命令或构建应用。

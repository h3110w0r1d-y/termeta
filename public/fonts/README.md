# 内置终端字体资源

核查日期：2026-10-15。共12个字体家族、35个字体文件，原始字体合计9,511,856字节。字体字节保持上游原样，不进行子集化、重命名字体内部名称或格式转换。每个目录保留相应许可证；Meslo 另保留作者 README 的版权、商标与许可说明，Droid 保留 AOSP 字体 NOTICE。

| 字体 | 版本 | 官方发布/分发来源 | 许可证 |
| --- | --- | --- | --- |
| DejaVu Sans Mono | 2.37 | [来源](https://github.com/dejavu-fonts/dejavu-fonts/releases/tag/version_2_37) | Bitstream Vera + DejaVu public domain |
| Droid Sans Mono | 1.00 | [来源](https://github.com/aosp-mirror/platform_frameworks_base/tree/1cdfff555f4a21f71ccc978290e2e212e2f8b168/data/fonts) | Apache-2.0 |
| Ubuntu Mono | 0.83 | [来源](https://assets.ubuntu.com/v1/0cef8205-ubuntu-font-family-0.83.zip) | Ubuntu Font Licence 1.0 |
| Anonymous Pro | 1.003 | [来源](https://github.com/google/fonts/tree/b7c8331cf620346575487ef8db0e2c729f014675/ofl/anonymouspro) | OFL-1.1 |
| Inconsolata | 3.000 | [来源](https://github.com/googlefonts/Inconsolata/releases/tag/v3.000) | OFL-1.1 |
| Fira Mono | 3.206 (Fira release 4.202) | [来源](https://github.com/mozilla/Fira/tree/4.202) | OFL-1.1 |
| Source Code Pro | 2.042 upright / 1.062 italic | [来源](https://github.com/adobe-fonts/source-code-pro/releases/tag/2.042R-u/1.062R-i/1.026R-vf) | OFL-1.1 |
| PT Mono | 1.001W OFL | [来源](https://github.com/google/fonts/tree/b7c8331cf620346575487ef8db0e2c729f014675/ofl/ptmono) | OFL-1.1 |
| Cascadia Code | 2407.24 | [来源](https://github.com/microsoft/cascadia-code/releases/tag/v2407.24) | OFL-1.1 |
| Fira Code | 6.2 | [来源](https://github.com/tonsky/FiraCode/releases/tag/6.2) | OFL-1.1 |
| JetBrains Mono | 2.304 | [来源](https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304) | OFL-1.1 |
| Meslo | 1.2.1 | [来源](https://github.com/andreberg/Meslo-Font/tree/09a431d546d211130352c28eb0466e5d7d5aeaf0/dist/v1.2.1) | Apache-2.0 (author README) |

可用 GitHub Release 的项目取核查时 latest 正式发布；Droid 无独立 Release，使用 AOSP 官方 GitHub 镜像当日 main 的固定提交。Anonymous Pro 的作者官网拒绝抓取、PT Mono 官网下载证书校验失败，二者使用 Google Fonts 官方仓库当日固定提交的开源分发，不将未经验证的网站版本宣称为更高版本。Ubuntu Mono 采用 Canonical 官方经典字体包0.83，不用 Ubuntu Sans Mono 替代；Meslo 采用作者 dist 最新1.2.1中的 LG S 小行距四字型，不用 Nerd Font 派生版替代。

完整来源、固定提交、各字体内部版本、字重/样式、版权、文件大小与 SHA-256 见 [manifest.json](manifest.json)。字体通过应用内 CSS 别名使用，不覆盖系统字体，不需要用户安装或在线下载。没有独立粗体/斜体的家族交由浏览器合成对应样式；Fira Code/Cascadia Code 等字体文件包含连字信息，不代表终端已实现连字渲染。

资源位于 public，Vite 打包时原样复制全部字体、manifest 与许可证，加载时按需请求当前所选家族，不预加载全部字体。运行时不请求本文外部链接；CSP 继续只允许本地字体。

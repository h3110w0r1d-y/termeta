# 内置终端字体资源

核查日期：2026-10-19。共12个字体家族、36个字体文件，字体字节保持 Nerd Fonts 官方 ZIP 内的原样，不进行子集化、重命名字体内部名称或格式转换。除 PT Mono 外，其余家族使用 Nerd Fonts v3.5.1 对应的 Mono 变体；PT Mono 因官方下载页未提供而保留原版。每个目录保留相应许可证；Meslo 另保留作者 README 的版权、商标与许可说明，Droid 保留 AOSP 字体 NOTICE。

| 字体             | 上游版本   | 官方 Nerd Fonts 下载包                                                                               | 许可证                                |
| ---------------- | ---------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------- |
| DejaVu Sans Mono | 2.37       | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/DejaVuSansMono.zip)           | Bitstream Vera + DejaVu public domain |
| Droid Sans Mono  | 1.00-113   | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/DroidSansMono.zip)            | Apache-2.0                            |
| Ubuntu Mono      | 0.80       | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/UbuntuMono.zip)               | Ubuntu Font Licence 1.0               |
| Anonymous Pro    | 1.002      | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/AnonymousPro.zip)             | OFL-1.1                               |
| Inconsolata      | 3.000      | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/Inconsolata.zip)              | OFL-1.1                               |
| Fira Mono        | 3.206      | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/FiraMono.zip)                 | OFL-1.1                               |
| Source Code Pro  | 2.042      | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/SourceCodePro.zip)            | OFL-1.1                               |
| PT Mono          | 1.001W OFL | [原版来源](https://github.com/google/fonts/tree/b7c8331cf620346575487ef8db0e2c729f014675/ofl/ptmono) | OFL-1.1                               |
| Cascadia Code    | 2407.24    | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/CascadiaCode.zip)             | OFL-1.1                               |
| Fira Code        | 6.2        | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/FiraCode.zip)                 | OFL-1.1                               |
| JetBrains Mono   | 2.304      | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/JetBrainsMono.zip)            | OFL-1.1                               |
| Meslo            | 1.21       | [ZIP](https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/Meslo.zip)                    | Apache-2.0 (author README)            |

版本与来源按 Nerd Fonts v3.5.1 固定下载包锁定。PT Mono 未出现在 Nerd Fonts 官方下载页，因此继续使用原版常规体；Meslo 继续选择 LGS 小行距四字型的 Mono 变体。所有许可证及上游版权说明保持随包分发。

完整来源、固定下载包、各字体内部版本、字重/样式、版权、文件大小与 SHA-256 见 [manifest.json](manifest.json)。字体通过应用内 CSS 别名使用，不覆盖系统字体，不需要用户安装或在线下载。没有独立粗体/斜体的家族交由浏览器合成对应样式；Fira Code/Cascadia Code 等字体文件包含连字信息，不代表终端已实现连字渲染。

资源位于 public，Vite 打包时原样复制全部字体、manifest 与许可证，加载时按需请求当前所选家族，不预加载全部字体。运行时不请求本文外部链接；CSP 继续只允许本地字体。

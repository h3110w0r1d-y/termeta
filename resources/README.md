# 终端配色静态资源

日期：2026-10-15。`terminal-palettes.json` 包含从用户提供的 Termius 解包目录静态提取的64套终端配色，保留原名称与全部终端色值。前后端使用同一 JSON：前端映射为 Scheme，Rust 仅从中读取内置 ID 名单，避免维护两份白名单。默认方案为 Pro。

来源文件为 `ui-process/assets/reconnectSaga-871ae692.js`，另外三个进程包中的同一数组已做逐字节一致性核对。只截取21,621字节数组进行 TypeScript AST 数据解析，不运行解包程序，也不输出整行压缩 JS。来源文件与数组 SHA-256 见 [提取记录](terminal-palettes.provenance.json)。

映射：`colorPaletteOverrides` 的16色依原顺序保存为 ANSI；`backgroundColor` / `foregroundColor` / `cursorColor` / `selectionColor` 分别映射背景、前景、光标与选区。源数据未定义光标文字反色和失焦选区，分别沿用背景色与原选区色。`app.terminalUIColor` 不进入终端 Scheme。

提取工具为 [extract-terminal-palettes.mjs](../scripts/extract-terminal-palettes.mjs)，接收输入 JS 路径与输出 JSON 路径；仅接受对象/数组/字符串字面量，拒绝表达式、方法与函数调用。脚本打印摘要，不打印完整源码。提取工具依赖仓库已有 TypeScript，不增加依赖。

旧项目预设已移除；旧暗色预设引用回退为 Nord Dark，旧浅色引用回退为 Nord Light，仅归一读取结果，下次主动保存才持久化。同名 Ocean 采用提取后的值。用户已保存的自定义配色不删除。终端仍通过原 xterm 主题接口热应用，界面名称与色块继续分行。

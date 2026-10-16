# 静态资源来源

`terminal-palettes.json` 包含从用户提供的 Termius 解包目录静态提取的64套终端配色，保留原名称与全部终端色值。前后端使用同一 JSON：前端映射为 Scheme，Rust 仅从中读取内置 ID 名单，避免维护两份白名单。默认方案为 Pro。

来源文件为 `ui-process/assets/reconnectSaga-871ae692.js`，另外三个进程包中的同一数组已做逐字节一致性核对。只截取21,621字节数组进行 TypeScript AST 数据解析，不运行解包程序，也不输出整行压缩 JS。来源文件与数组 SHA-256 见 [提取记录](terminal-palettes.provenance.json)。

映射：`colorPaletteOverrides` 的16色依原顺序保存为 ANSI；`backgroundColor` / `foregroundColor` / `cursorColor` / `selectionColor` 分别映射背景、前景、光标与选区。源数据未定义光标文字反色和失焦选区，分别沿用背景色与原选区色。`app.terminalUIColor` 不进入终端 Scheme。

提取工具为 [extract-terminal-palettes.mjs](../scripts/extract-terminal-palettes.mjs)，接收输入 JS 路径与输出 JSON 路径；仅接受对象/数组/字符串字面量，拒绝表达式、方法与函数调用。脚本打印摘要，不打印完整源码。提取工具依赖仓库已有 TypeScript，不增加依赖。

读取旧预设引用时，暗色回退Nord Dark、浅色回退Nord Light；同名Ocean采用当前提取值。归一结果在主动保存时持久化，不删除自定义方案。终端通过现有xterm主题接口热应用，行为与限制见 [配色说明](../docs/15-terminal-themes.md)。

## 主机搜索拼音表

按用户指定，从本地 roco-helper-web 项目的 `src/assets/conf/PINYIN_CONF.ts` 复制20903条Unicode码点到拼音候选的配置至 [PINYIN_CONF.ts](../src/assets/conf/PINYIN_CONF.ts)，仅将文件头注释改为中文，配置数据一致。匹配逻辑参考该项目 `src/modules/big-map/petNameSearch.ts`，适配为 [主机搜索工具](../src/modules/hosts/search.ts)，支持原文、全拼、首字母、多音字、部分音节和中英/拼音混合输入；不引入上游业务数据或新依赖。复制日期与原始/本地摘要见[来源记录](pinyin.provenance.json)。

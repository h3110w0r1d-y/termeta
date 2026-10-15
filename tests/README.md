# 工作区布局测试

`workspace-layout.test.mjs` 使用 Node 内置测试与断言模块，不新增依赖。需要 Node **22.18+**，通过其原生 TypeScript 类型擦除能力直接导入 `src/modules/workspace/layout.ts`。

遵循仓库约束，需用户明确授权后才执行：

```sh
node --test tests/workspace-layout.test.mjs
```

当前仅完成源码、语法与格式静态检查，**尚未执行测试**。这些用例只验证纯数据布局规则，不代表 SSH、WebView、拖动手势或桌面集成已通过验证。

迁移用例同时覆盖同树窗格重排、跨组来源收缩、单叶来源清除，以及自身/无效/重复目标拒绝；断言原树不变且每个终端只出现一次。

`terminal-broadcast.test.mjs`覆盖输入路由、目标去重、设备应答/鼠标隔离、光标模式、粘贴/滚轮递归保护，以及固定xterm来源桥接的能力降级/清理。仅语法静态检查，未执行；用户明确授权后可用 `node --test tests/terminal-broadcast.test.mjs`，需要Node22.18+。这些模拟事件用例不能替代xterm/WebView/真实SSH/vim互通验证。

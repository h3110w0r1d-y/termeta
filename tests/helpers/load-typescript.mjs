import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import ts from "typescript";

// 显式注入依赖，避免 Node 加载纯逻辑用例时进入界面或桌面运行环境。
export function loadTypeScript(url, dependencies) {
  const { outputText } = ts.transpileModule(readFileSync(url, "utf8"), {
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.CommonJS,
    },
  });
  const exports = {};
  runInNewContext(outputText, {
    exports,
    require(id) {
      assert.ok(Object.hasOwn(dependencies, id), `未提供依赖：${id}`);
      return dependencies[id];
    },
  });
  return exports;
}

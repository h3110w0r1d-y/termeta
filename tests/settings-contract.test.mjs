import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { runInNewContext } from "node:vm";
import * as vue from "vue";
import ts from "typescript";

function load(path, dependencies) {
  const { outputText } = ts.transpileModule(
    readFileSync(new URL(path, import.meta.url), "utf8"),
    {
      compilerOptions: {
        target: ts.ScriptTarget.ES2022,
        module: ts.ModuleKind.CommonJS,
      },
    },
  );
  const exports = {};
  runInNewContext(outputText, {
    exports,
    navigator: { platform: "MacIntel", languages: ["en-US"] },
    require(id) {
      assert.ok(Object.hasOwn(dependencies, id), `未提供依赖：${id}`);
      return dependencies[id];
    },
  });
  return exports;
}

// 隔离组件语言包与浏览器环境；字体、语言归一和默认绑定继续执行真实规则。
const i18n = load("../src/i18n/index.ts", {
  vue,
  "naive-ui": {},
  "./messages": {
    messages: Object.fromEntries(
      ["zh-CN", "zh-TW", "en-US", "ja-JP", "ko-KR"].map((id) => [id, {}]),
    ),
  },
});
const { normalizeSettings, hostSchemePatch } = load(
  "../src/modules/settings/state.ts",
  {
    "@/i18n": i18n,
    "@/services/commands/registry": load(
      "../src/services/commands/registry.ts",
      { "@/i18n": i18n },
    ),
    "@/theme/schemes": { defaultSchemeId: "pro" },
    "./fonts": load("../src/modules/settings/fonts.ts", {}),
  },
);
const contract = JSON.parse(
  readFileSync(new URL("./fixtures/settings-contract.json", import.meta.url)),
);
const plain = (value) => JSON.parse(JSON.stringify(value));
const snapshot = (fields) => ({ ...contract.baseline, ...fields });

for (const entry of contract.cases) {
  test(`设置兼容契约：${entry.name}`, () => {
    let previous = snapshot(entry.normalized);
    for (const step of [entry, ...entry.patches]) {
      // Rust 用例将真实归一/合并输出逐项对照 normalized；此端接续同一 wire 快照。
      const wire = snapshot(step.normalized);
      const original = plain(wire);
      assert.deepEqual(plain(normalizeSettings(wire)), {
        ...contract.baseline,
        ...contract.uiDefaults,
        ...step.normalized,
        ...step.ui,
      });
      assert.deepEqual(wire, original, "读取投影不得原地改写持久化快照");
      if (step.patch?.hostSchemes)
        assert.deepEqual(
          plain(hostSchemePatch(wire.hostSchemes, previous.hostSchemes)),
          step.patch.hostSchemes,
          "删除覆盖只提交指定 ID 的 null，保留其他主机覆盖",
        );
      previous = wire;
    }
  });
}

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { runInNewContext } from "node:vm";
import ts from "typescript";

function deferred() {
  let resolve;
  const promise = new Promise((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

/** 使用真实 Registry 的关闭流程，只替换归档端口与终端视图。 */
function fixture(archive) {
  const workbench = {
    terminals: [],
    closingTerminalIds: [],
    remove(id) {
      this.terminals = this.terminals.filter((item) => item.terminalId !== id);
    },
  };
  const entries = new Map();
  const dependencies = {
    "@/i18n": {},
    "@/services/desktop/client": { errorMessage: (error) => error.message },
    vue: {},
    "./inputOrigin": {},
    "./inputRouter": { createInputRouter: () => ({}) },
    "./paste": {},
    "../workbench/layout": {},
    "../settings/store": {},
    "@/theme/schemes": {},
    "../workbench/store": { useWorkbenchStore: () => workbench },
    "./runtime/stream": {},
    "./runtime/session": {
      connect: async () => {},
      archiveAndDispose: archive,
    },
    "./runtime/view": {
      createTerminalEntry(terminal) {
        const entry = { terminal };
        entries.set(terminal.terminalId, entry);
        return entry;
      },
      mountTerminalView() {},
    },
  };
  const source = readFileSync(
    new URL("../src/modules/terminal/registry.ts", import.meta.url),
    "utf8",
  );
  const { outputText } = ts.transpileModule(source, {
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
  const registry = exports.terminalRegistry;
  function mount(id) {
    const terminal = { terminalId: id, error: "" };
    workbench.terminals.push(terminal);
    registry.mount(terminal, {});
    return terminal;
  }
  return { registry, workbench, entries, mount };
}

test("普通关闭与退出同时请求同一视图时只归档一次，提交前保留布局", async () => {
  const pending = deferred();
  let saves = 0;
  const { registry, workbench, mount } = fixture(async () => {
    saves++;
    await pending.promise;
  });
  mount("terminal-a");
  const tabClose = registry.closeAndRemove("terminal-a");
  const windowClose = registry.closeMany(["terminal-a"]);
  await Promise.resolve();
  assert.equal(saves, 1);
  assert.equal(workbench.terminals.length, 1);
  assert.equal(workbench.closingTerminalIds.includes("terminal-a"), true);
  pending.resolve();
  await Promise.all([tabClose, windowClose]);
  assert.equal(workbench.terminals.length, 0);
  assert.equal(workbench.closingTerminalIds.length, 0);
});

test("归档失败保留相同视图及历史引用，清除关闭占用后可重试", async () => {
  let saves = 0;
  const { registry, workbench, entries, mount } = fixture(async () => {
    if (++saves === 1) throw new Error("保存失败");
  });
  const terminal = mount("terminal-a");
  const entry = entries.get("terminal-a");
  await assert.rejects(registry.closeAndRemove("terminal-a"), /保存失败/);
  assert.equal(workbench.terminals[0], terminal);
  assert.equal(entries.get("terminal-a"), entry);
  assert.equal(terminal.error, "保存失败");
  assert.equal(entry.closing, undefined);
  assert.equal(workbench.closingTerminalIds.length, 0);
  await registry.closeAndRemove("terminal-a");
  assert.equal(saves, 2);
  assert.equal(workbench.terminals.length, 0);
});

test("批量关闭最多两路归档，单项失败不阻止其余提交", async () => {
  let active = 0;
  let peak = 0;
  const firstPair = deferred();
  const { registry, workbench, mount } = fixture(async (_, terminal) => {
    active++;
    peak = Math.max(peak, active);
    try {
      await firstPair.promise;
      if (terminal.terminalId === "terminal-1") throw new Error("保存失败");
    } finally {
      active--;
    }
  });
  const ids = Array.from({ length: 20 }, (_, index) => `terminal-${index}`);
  ids.forEach(mount);
  const closing = Promise.all([
    registry.closeMany(ids.slice(0, 10)),
    registry.closeMany(ids.slice(10)),
  ]);
  await Promise.resolve();
  assert.equal(active, 2);
  firstPair.resolve();
  const results = (await closing).flat();
  assert.equal(peak, 2);
  assert.equal(
    results.filter((result) => result.status === "fulfilled").length,
    19,
  );
  assert.equal(workbench.terminals.length, 1);
  assert.equal(workbench.terminals[0].terminalId, "terminal-1");
});

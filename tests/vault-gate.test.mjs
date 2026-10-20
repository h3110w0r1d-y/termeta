import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { runInNewContext } from "node:vm";
import * as vue from "vue";
import { compileScript, parse } from "vue/compiler-sfc";
import ts from "typescript";

const unlocked = {
  mode: "unlocked",
  keychainWarning: null,
  cleanupWarning: null,
};
function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

async function flush() {
  for (let turn = 0; turn < 20; turn++) {
    await Promise.resolve();
    await vue.nextTick();
  }
}

// 在内存宿主中使用真实 Vue 调度和组件脚本，只替换 IPC 与界面依赖。
function evaluate(source, filename, dependencies) {
  const { outputText } = ts.transpileModule(source, {
    fileName: filename,
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.CommonJS,
    },
  });
  const exports = {};
  runInNewContext(
    outputText,
    {
      exports,
      require(id) {
        assert.ok(Object.hasOwn(dependencies, id), `未提供依赖：${id}`);
        return dependencies[id];
      },
      TextEncoder,
      setTimeout,
      clearTimeout,
    },
    { filename },
  );
  return exports;
}

const renderer = vue.createRenderer({
  patchProp() {},
  insert() {},
  remove() {},
  createElement: () => ({}),
  createText: () => ({}),
  createComment: () => ({}),
  setText() {},
  setElementText() {},
  parentNode: () => null,
  nextSibling: () => null,
});

function mountSetup(setup) {
  let state;
  const app = renderer.createApp({
    setup() {
      state = setup();
      return () => null;
    },
  });
  app.mount({});
  return { state, unmount: () => app.unmount() };
}

function fixture(
  t,
  {
    status = async () => unlocked,
    unlock = async () => unlocked,
    initialize = async () => {},
  } = {},
) {
  let vaultReads = 0;
  const unlockRequests = [];
  const desktop = {
    vaultStatus() {
      vaultReads++;
      return status();
    },
    unlockVault(password, setup) {
      unlockRequests.push({ password, setup });
      return unlock(password, setup);
    },
  };
  const client = { desktop, errorMessage: (error) => error.message };
  const i18n = { t: (key) => key };
  const { descriptor } = parse(
    readFileSync(
      new URL("../src/components/VaultGate.vue", import.meta.url),
      "utf8",
    ),
  );
  const script = compileScript(descriptor, { id: "vault-gate-regression" });
  const component = evaluate(script.content, "VaultGate.ts", {
    vue,
    "../i18n": i18n,
    "naive-ui": {},
    "../services/forms/rules": { validateForm: async () => true },
    "./ApplicationMenu.vue": {},
    "./WindowControls.vue": {},
    "../services/commands/registry": { isMac: false },
    "../services/desktop/client": client,
  }).default;
  const mounted = mountSetup(() =>
    component.setup({ initialize }, { expose() {}, emit() {} }),
  );
  t.after(mounted.unmount);
  return {
    gate: mounted.state,
    unmount: mounted.unmount,
    unlockRequests,
    vaultReads: () => vaultReads,
  };
}

test("自动解锁后等待工作台初始化完成才放行", async (t) => {
  const initialization = deferred();
  let initialized = 0;
  const f = fixture(t, {
    initialize: () => {
      initialized++;
      return initialization.promise;
    },
  });
  await flush();
  assert.equal(f.vaultReads(), 1);
  assert.equal(initialized, 1);
  assert.equal(f.gate.showPassword.value, false);
  assert.equal(f.gate.ready.value, false);
  assert.equal(f.gate.busy.value, true);
  initialization.resolve();
  await flush();
  assert.equal(f.gate.ready.value, true);
  assert.equal(f.unlockRequests.length, 0);
});

test("全新保险库确认创建后解锁，提交结束清空密码", async (t) => {
  let mode = "setup";
  const f = fixture(t, {
    status: async () => ({ ...unlocked, mode }),
    unlock: async () => {
      mode = "unlocked";
      return unlocked;
    },
  });
  await flush();
  assert.equal(f.gate.setup.value, true);
  assert.equal(f.gate.ready.value, false);
  f.gate.password.value = "new-vault-password";
  f.gate.confirmation.value = "new-vault-password";
  await f.gate.submit();
  assert.deepEqual(f.unlockRequests, [
    { password: "new-vault-password", setup: true },
  ]);
  assert.equal(f.gate.ready.value, true);
  assert.equal(f.gate.password.value, "");
  assert.equal(f.gate.confirmation.value, "");
});

test("既有锁定库要求本机密码，失败保留门禁并可重试", async (t) => {
  let mode = "locked";
  let attempts = 0;
  const f = fixture(t, {
    status: async () => ({ ...unlocked, mode }),
    unlock: async () => {
      if (++attempts === 1) throw new Error("模拟密码错误");
      mode = "unlocked";
      return unlocked;
    },
  });
  await flush();
  assert.equal(f.gate.setup.value, false);
  assert.equal(f.gate.showPassword.value, true);
  f.gate.password.value = "wrong-password";
  await f.gate.submit();
  assert.equal(f.gate.ready.value, false);
  assert.equal(f.gate.error.value, "模拟密码错误");
  assert.equal(f.gate.password.value, "");
  assert.equal(f.unlockRequests[0].setup, false);
  f.gate.password.value = "correct-password";
  await f.gate.submit();
  assert.equal(f.gate.ready.value, true);
  assert.equal(f.gate.error.value, "");
});

test("状态读取失败不会猜测为空库，重试后恢复明确状态", async (t) => {
  let reads = 0;
  const f = fixture(t, {
    status: async () => {
      if (++reads === 1) throw new Error("模拟读取失败");
      return { ...unlocked, mode: "locked" };
    },
  });
  await flush();
  assert.equal(f.gate.ready.value, false);
  assert.equal(f.gate.showPassword.value, false);
  assert.equal(f.gate.title.value, "vault.access.failedTitle");
  assert.equal(f.gate.error.value, "模拟读取失败");
  await f.gate.inspect();
  assert.equal(f.gate.showPassword.value, true);
  assert.equal(f.gate.setup.value, false);
});

test("工作台初始化失败不放行，重试重新初始化并保留缓存警告", async (t) => {
  let attempts = 0;
  const f = fixture(t, {
    status: async () => ({ ...unlocked, keychainWarning: "模拟缓存失败" }),
    initialize: async () => {
      if (++attempts === 1) throw new Error("模拟初始化失败");
    },
  });
  await flush();
  assert.equal(f.gate.ready.value, false);
  assert.equal(f.gate.showPassword.value, false);
  assert.equal(f.gate.error.value, "模拟初始化失败");
  await f.gate.inspect();
  assert.equal(attempts, 2);
  assert.equal(f.gate.ready.value, true);
  assert.equal(f.gate.warning.value, "模拟缓存失败");
});

test("窗口卸载后迟到状态不会再初始化工作台", async (t) => {
  const status = deferred();
  let initialized = 0;
  const f = fixture(t, {
    status: () => status.promise,
    initialize: async () => {
      initialized++;
    },
  });
  f.unmount();
  status.resolve(unlocked);
  await flush();
  assert.equal(initialized, 0);
  assert.equal(f.gate.ready.value, false);
});

test("窗口卸载后迟到初始化不能放行", async (t) => {
  const initialization = deferred();
  const f = fixture(t, { initialize: () => initialization.promise });
  await flush();
  f.unmount();
  initialization.resolve();
  await flush();
  assert.equal(f.gate.ready.value, false);
});

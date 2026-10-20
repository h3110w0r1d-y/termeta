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
const signedOut = { user: null, expiresAt: null };
const savedAccount = {
  user: { id: "account-a", email: "a@example.invalid" },
  expiresAt: null,
};
const syncStatus = (phase) => ({
  phase,
  cloudHasData: phase !== "signedOut",
  lastSyncedAt: null,
  error: null,
});

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

function loadModule(path, dependencies) {
  return evaluate(
    readFileSync(new URL(path, import.meta.url), "utf8"),
    path,
    dependencies,
  );
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
  { account = savedAccount, initialize = async () => {} } = {},
) {
  let localUnlocked = false;
  let currentAccount = account;
  let accountChanged, syncChanged;
  let vaultReads = 0;
  const requests = [];
  const desktop = {
    async vaultStatus() {
      vaultReads++;
      localUnlocked = true;
      return unlocked;
    },
    async getAccountStatus() {
      return localUnlocked ? currentAccount : signedOut;
    },
    getOfficialSyncStatus() {
      const request = deferred();
      requests.push(request);
      return request.promise;
    },
    async onAccountChanged(callback) {
      accountChanged = callback;
      return () => {};
    },
    async onOfficialSyncChanged(callback) {
      syncChanged = callback;
      return () => {};
    },
  };
  const client = { desktop, errorMessage: (error) => error.message };
  const i18n = { t: (key) => key };
  const useAccount = loadModule("../src/modules/account/useAccount.ts", {
    vue,
    "../../i18n": i18n,
    "../../services/desktop/client": client,
  });
  const useOfficialSync = loadModule(
    "../src/modules/account/useOfficialSync.ts",
    {
      vue,
      "../../services/desktop/client": client,
    },
  );
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
    "../modules/account/useAccount": useAccount,
    "../modules/account/useOfficialSync": useOfficialSync,
    "../modules/account/BrowserAccountLogin.vue": {},
  }).default;
  const mounted = mountSetup(() =>
    component.setup({ initialize }, { expose() {}, emit() {} }),
  );
  t.after(mounted.unmount);
  return {
    gate: mounted.state,
    requests,
    vaultReads: () => vaultReads,
    syncChanged: () => syncChanged(),
    async switchAccount(account) {
      currentAccount = account;
      accountChanged();
      await flush();
    },
  };
}

test("已保存账号自动恢复后直接进入，自身账号更新不重复检查", async (t) => {
  let initialized = 0;
  const f = fixture(t, {
    initialize: async () => {
      initialized++;
    },
  });
  await flush();
  assert.equal(f.requests.length, 1);
  assert.equal(f.vaultReads(), 1);
  assert.equal(f.gate.showPassword.value, false);
  assert.equal(f.gate.title.value, "vault.access.checkingTitle");
  assert.equal(f.gate.ready.value, false);
  f.requests[0].resolve(syncStatus("ready"));
  await flush();
  assert.equal(f.gate.ready.value, true);
  assert.equal(initialized, 1);
  assert.equal(f.vaultReads(), 1);
});

test("同窗口并发读取共用正在进行的同步状态请求", async (t) => {
  const f = fixture(t);
  await flush();
  const first = f.gate.refreshSync();
  const second = f.gate.refreshSync();
  assert.equal(first, second);
  assert.equal(f.requests.length, 1);
  f.requests[0].resolve(syncStatus("ready"));
  await Promise.all([first, second]);
  await flush();
  assert.equal(f.gate.ready.value, true);
});

test("查询期间的状态通知不会丢失，也不会放行已经过期的 ready", async (t) => {
  const f = fixture(t);
  await flush();
  f.syncChanged();
  f.requests[0].resolve(syncStatus("ready"));
  await flush();
  assert.equal(f.requests.length, 2);
  assert.equal(f.gate.ready.value, false);
  f.requests[1].resolve(syncStatus("needsPassword"));
  await flush();
  assert.equal(f.gate.showPassword.value, true);
  assert.equal(f.gate.explanation.value, "account.sync.unlockDescription");
  assert.equal(f.gate.ready.value, false);
});

test("恢复期间切换账号后丢弃旧账号的同步结果", async (t) => {
  const f = fixture(t);
  await flush();
  await f.switchAccount({
    user: { id: "account-b", email: "b@example.invalid" },
    expiresAt: null,
  });
  f.requests[0].resolve(syncStatus("ready"));
  await flush();
  assert.equal(f.requests.length, 2);
  assert.equal(f.gate.ready.value, false);
  f.requests[1].resolve(syncStatus("needsPassword"));
  await flush();
  assert.equal(f.gate.account.value.user.id, "account-b");
  assert.equal(f.gate.showPassword.value, true);
  assert.equal(f.gate.ready.value, false);
});

test("工作台初始化期间重新要求密码，迟到初始化不能放行", async (t) => {
  const initialization = deferred();
  const f = fixture(t, { initialize: () => initialization.promise });
  await flush();
  f.requests[0].resolve(syncStatus("ready"));
  await flush();
  f.syncChanged();
  f.requests[1].resolve(syncStatus("needsPassword"));
  await flush();
  initialization.resolve();
  await flush();
  assert.equal(f.gate.initialized.value, true);
  assert.equal(f.gate.ready.value, false);
  assert.equal(f.gate.showPassword.value, true);
});

test("后续就绪通知会自动重新检查并进入工作台", async (t) => {
  const f = fixture(t);
  await flush();
  f.requests[0].resolve(syncStatus("needsPassword"));
  await flush();
  f.syncChanged();
  f.requests[1].resolve(syncStatus("ready"));
  await flush();
  assert.equal(f.requests.length, 3);
  f.requests[2].resolve(syncStatus("ready"));
  await flush();
  assert.equal(f.gate.ready.value, true);
});

test("查询失败显示恢复失败，重试后才按明确状态展示密码表单", async (t) => {
  const f = fixture(t);
  await flush();
  f.requests[0].reject(new Error("模拟读取失败"));
  await flush();
  assert.equal(f.gate.ready.value, false);
  assert.equal(f.gate.showPassword.value, false);
  assert.equal(f.gate.title.value, "vault.access.failedTitle");
  assert.equal(f.gate.explanation.value, "vault.access.failedDescription");
  const retry = f.gate.inspect();
  await flush();
  f.requests[1].resolve(syncStatus("needsPassword"));
  await retry;
  assert.equal(f.gate.showPassword.value, true);
  assert.equal(f.gate.ready.value, false);
});

test("本地模式自动解锁后进入工作台，不查询官方同步", async (t) => {
  const f = fixture(t, { account: signedOut });
  await flush();
  assert.equal(f.gate.ready.value, true);
  assert.equal(f.requests.length, 0);
});

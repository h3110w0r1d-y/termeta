import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { runInNewContext } from "node:vm";
import * as vue from "vue";
import * as pinia from "pinia";
import ts from "typescript";

// 执行真实 Store/组合函数，仅替换 IPC 和生命周期登记以控制异步返回顺序。
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
    require(id) {
      assert.ok(Object.hasOwn(dependencies, id), `未提供依赖：${id}`);
      return dependencies[id];
    },
  });
  return exports;
}

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

for (const action of [
  "save",
  "remove",
  "saveWorkspace",
  "saveGroup",
  "arrangeHosts",
]) {
  test(`主机旧快照不覆盖已完成的 ${action}`, async () => {
    pinia.setActivePinia(pinia.createPinia());
    const old = deferred();
    const saved = {
      id: "host",
      name: "新名称",
      revision: 2,
      sortOrder: 0,
    };
    const group = { id: "group", name: "新分组", revision: 2 };
    const desktop = {
      listHostInventory: () => old.promise,
      saveHost: async () => saved,
      deleteHost: async () => {},
      saveWorkspace: async () => saved,
      saveGroup: async () => group,
      arrangeHosts: async () => ({ hosts: [saved], groups: [group] }),
    };
    const { useHosts } = load("../src/modules/hosts/store.ts", {
      vue,
      pinia,
      "@/services/desktop/client": { desktop },
      "./ordering": load("../src/modules/hosts/ordering.ts", {}),
    });
    const store = useHosts();
    store.hosts = [{ ...saved, name: "旧名称", revision: 1 }];
    const loading = store.load();
    if (action === "save") await store.save({}, 1);
    else if (action === "remove") await store.remove(store.hosts[0]);
    else if (action === "saveWorkspace") await store.saveWorkspace({});
    else if (action === "arrangeHosts") await store.arrangeHosts(["host"]);
    else await store.saveGroup({}, 1);
    old.resolve({
      hosts: [{ ...saved, name: "旧名称", revision: 1 }],
      groups: [],
    });
    await loading;
    if (action === "remove") assert.equal(store.hosts.length, 0);
    else if (action === "saveGroup")
      assert.equal(store.groups[0].name, "新分组");
    else assert.equal(store.hosts[0].revision, 2);
  });
}

function settingsFixture({ subscribe, read }) {
  let dispose;
  const { useSettingsSync } = load(
    "../src/modules/settings/useSettingsSync.ts",
    {
      vue: {
        ...vue,
        onBeforeUnmount(handler) {
          dispose = handler;
        },
      },
      "./store": { useSettings: () => ({ load: read }) },
      "@/services/desktop/client": {
        desktop: { onSettingsChanged: subscribe },
        errorMessage: String,
      },
    },
  );
  return { sync: useSettingsSync(), dispose: () => dispose() };
}

test("设置首次读取失败后重试不重复订阅，并发 start 共用一次初始化", async () => {
  let subscriptions = 0,
    reads = 0,
    releases = 0,
    changed;
  const fixture = settingsFixture({
    async subscribe(handler) {
      subscriptions++;
      changed = handler;
      return () => releases++;
    },
    async read() {
      if (++reads === 1) throw new Error("模拟读取失败");
    },
  });
  const first = fixture.sync.start();
  assert.equal(first, fixture.sync.start());
  await assert.rejects(first);
  await fixture.sync.start();
  changed();
  // 排空串行读取链，避免依赖计时器或真实 IPC。
  for (let index = 0; index < 12; index++) await Promise.resolve();
  assert.equal(subscriptions, 1);
  assert.equal(reads, 3);
  fixture.dispose();
  assert.equal(releases, 1);
});

test("设置订阅失败可以重试，卸载后的迟到订阅立即释放", async () => {
  const subscription = deferred();
  let calls = 0,
    reads = 0,
    releases = 0;
  const fixture = settingsFixture({
    subscribe() {
      return ++calls === 1
        ? Promise.reject(new Error("模拟订阅失败"))
        : subscription.promise;
    },
    async read() {
      reads++;
    },
  });
  await assert.rejects(fixture.sync.start());
  const starting = fixture.sync.start();
  fixture.dispose();
  subscription.resolve(() => releases++);
  await starting;
  assert.equal(calls, 2);
  assert.equal(reads, 0);
  assert.equal(releases, 1);
});

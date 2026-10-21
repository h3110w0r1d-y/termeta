import assert from "node:assert/strict";
import test from "node:test";
import { effectScope, nextTick, reactive } from "vue";
import { useSftpDirectoryFollow } from "../src/modules/files/useSftpDirectoryFollow.ts";

function fixture(context) {
  context.mock.timers.enable({ apis: ["setTimeout"] });
  const state = reactive({
    enabled: false,
    sessionId: "one",
    path: "/a",
    available: true,
  });
  const calls = [];
  const scope = effectScope();
  scope.run(() =>
    useSftpDirectoryFollow({
      enabled: () => state.enabled,
      sessionId: () => state.sessionId,
      path: () => state.path,
      available: () => state.available,
      navigate: async (path) => {
        calls.push(path);
      },
    }),
  );
  context.after(() => scope.stop());
  async function settle() {
    await nextTick();
    context.mock.timers.tick(120);
    await nextTick();
  }
  return { state, calls, scope, settle };
}

test("启用后立即跟随已知目录，忙碌或隐藏期间合并为最新目标", async (context) => {
  const { state, calls, settle } = fixture(context);
  await settle();
  assert.deepEqual(calls, []);
  state.enabled = true;
  await settle();
  assert.deepEqual(calls, ["/a"]);
  state.available = false;
  state.path = "/b";
  await settle();
  state.path = "/c";
  await settle();
  assert.deepEqual(calls, ["/a"]);
  state.available = true;
  await settle();
  assert.deepEqual(calls, ["/a", "/c"]);
});

test("禁用、断线和卸载丢弃待跳转；新连接等待新目录", async (context) => {
  const { state, calls, scope, settle } = fixture(context);
  state.enabled = true;
  await nextTick();
  state.enabled = false;
  await settle();
  assert.deepEqual(calls, []);
  state.enabled = true;
  await nextTick();
  state.sessionId = undefined;
  state.path = undefined;
  await settle();
  state.sessionId = "two";
  await settle();
  assert.deepEqual(calls, []);
  state.path = "/new";
  await settle();
  assert.deepEqual(calls, ["/new"]);
  state.path = "/disposed";
  await nextTick();
  scope.stop();
  await settle();
  assert.deepEqual(calls, ["/new"]);
});

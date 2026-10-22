import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { runInNewContext } from "node:vm";
import ts from "typescript";
import { isSessionTerminal } from "../src/modules/sessions/control.ts";

// 挂载真实终端视图路由，只替换 xterm、DOM 和重连入口，不启动应用或连接主机。
function fixture(state = "disconnected") {
  class Element {
    recorder = false;
    closest() {
      return this.recorder ? this : null;
    }
  }
  const textarea = new Element();
  const terminal = { terminalId: "terminal-a", state, error: "" };
  const workbench = { activeTerminal: terminal };
  const document = {
    activeElement: textarea,
    blocked: false,
    querySelector() {
      return this.blocked ? {} : null;
    },
  };
  let keyHandler;
  let requests = 0;
  let routed = false;
  const term = {
    textarea,
    options: {},
    parser: { registerOscHandler() {} },
    onTitleChange() {},
    open() {},
    attachCustomKeyEventHandler(handler) {
      keyHandler = handler;
    },
    attachCustomWheelEventHandler() {},
  };
  const entry = {
    terminal,
    term,
    host: { clientWidth: 0, addEventListener() {} },
    observer: { observe() {} },
    fontRequest: "14px monospace",
    disposed: false,
  };
  const dependencies = {
    "@xterm/xterm": {},
    "@xterm/addon-fit": {},
    "@xterm/addon-search": {},
    "@xterm/addon-web-links": {},
    "@/i18n": {},
    "@/services/desktop/client": { errorMessage: (error) => error.message },
    "@/services/commands/registry": {
      commands: { route: () => routed },
      isMac: false,
    },
    "@/modules/hosts/store": {},
    "@/modules/settings/store": {
      useSettings: () => ({
        value: { fontSize: 14, scrollback: 5000, keymap: { bindings: [] } },
      }),
    },
    "@/modules/settings/fonts": { terminalFontFamily: () => "monospace" },
    "@/modules/settings/themes": { resolveTerminalScheme: () => ({}) },
    "@/modules/workbench/store": { useWorkbenchStore: () => workbench },
    "@/modules/sessions/control": { isSessionTerminal },
    "../activity": {},
    "../workingDirectory": {},
    "../inputOrigin": { observeTerminalInput: () => ({}) },
    "../mouseActions": { bindTerminalMouseActions: () => () => {} },
    "../theme": { applyTerminalTheme() {} },
  };
  const { outputText } = ts.transpileModule(
    readFileSync(
      new URL("../src/modules/terminal/runtime/view.ts", import.meta.url),
      "utf8",
    ),
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
    document,
    Element,
    require(id) {
      assert.ok(Object.hasOwn(dependencies, id), `未提供依赖：${id}`);
      return dependencies[id];
    },
  });
  exports.mountTerminalView(entry, terminal, {
    data() {
      assert.fail("重连不能生成终端输入");
    },
    paste: async () => {},
    pasteClipboard: async () => {},
    reconnect() {
      requests++;
      entry.connection = new Promise(() => {});
      return entry.connection;
    },
  });
  function key(overrides = {}) {
    const event = {
      type: "keydown",
      key: "Enter",
      ctrlKey: false,
      metaKey: false,
      altKey: false,
      shiftKey: false,
      repeat: false,
      isComposing: false,
      keyCode: 13,
      defaultPrevented: false,
      stopped: false,
      getModifierState: () => false,
      preventDefault() {
        this.defaultPrevented = true;
      },
      stopPropagation() {
        this.stopped = true;
      },
      ...overrides,
    };
    return { event, passthrough: keyHandler(event) };
  }
  return {
    key,
    entry,
    terminal,
    workbench,
    document,
    textarea,
    get requests() {
      return requests;
    },
    route() {
      routed = true;
    },
  };
}

test("关闭、失败和断开终态回车只重连一次，并阻止 xterm 编码或广播", () => {
  for (const state of ["closed", "failed", "disconnected"]) {
    const f = fixture(state);
    const result = f.key();
    assert.equal(result.passthrough, false);
    assert.equal(result.event.defaultPrevented, true);
    assert.equal(result.event.stopped, true);
    f.key();
    assert.equal(f.requests, 1);
  }
});

test("已连接及所有连接、认证、打开、关闭过渡态均不触发重连", () => {
  for (const state of [
    "created",
    "connecting",
    "awaitingHostKey",
    "authenticating",
    "awaitingAuth",
    "opening",
    "connected",
    "closing",
  ]) {
    const f = fixture(state);
    const result = f.key();
    assert.equal(result.passthrough, true, state);
    assert.equal(result.event.defaultPrevented, false, state);
    assert.equal(f.requests, 0, state);
  }
});

test("修饰回车、长按、IME、已消费事件、其他按键及 keyup 不触发重连", () => {
  for (const overrides of [
    { ctrlKey: true },
    { metaKey: true },
    { altKey: true },
    { shiftKey: true },
    { repeat: true },
    { isComposing: true },
    { keyCode: 229 },
    { defaultPrevented: true },
    { getModifierState: () => true },
    { key: "a" },
    { type: "keyup" },
  ]) {
    const f = fixture();
    f.key(overrides);
    assert.equal(f.requests, 0, JSON.stringify(overrides));
  }
});

test("焦点、录制、弹层、关闭和销毁边界阻止重连，自定义快捷键优先", () => {
  for (const block of [
    (f) => (f.workbench.activeTerminal = undefined),
    (f) => (f.workbench.activeTerminal = { terminalId: "terminal-b" }),
    (f) => (f.document.activeElement = {}),
    (f) => (f.document.blocked = true),
    (f) => (f.textarea.recorder = true),
    (f) => (f.entry.closing = Promise.resolve()),
    (f) => (f.entry.disposed = true),
    (f) => f.route(),
  ]) {
    const f = fixture();
    block(f);
    f.key();
    assert.equal(f.requests, 0);
  }
});

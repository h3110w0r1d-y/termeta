import assert from "node:assert/strict";
import test from "node:test";
import { bindTerminalMouseActions } from "../src/modules/terminal/mouseActions.ts";

function fixture() {
  const listeners = new Map();
  const actions = { 1: "none", 2: "none" };
  const options = {
    isMac: false,
    allowed: true,
    action: (button) => actions[button],
    canPaste: () => options.allowed,
    focus: () => focuses++,
    paste: async () => pastes++,
    onError: (error) => errors.push(error),
  };
  let pastes = 0;
  let focuses = 0;
  const errors = [];
  const host = {
    addEventListener(type, listener, capture) {
      assert.equal(capture, true);
      listeners.set(type, listener);
    },
    removeEventListener(type, listener, capture) {
      assert.equal(capture, true);
      assert.equal(listeners.get(type), listener);
      listeners.delete(type);
    },
  };
  const dispose = bindTerminalMouseActions(host, options);
  function event(type, button, overrides = {}) {
    const value = {
      button,
      ctrlKey: false,
      defaultPrevented: false,
      stopped: false,
      preventDefault() {
        this.defaultPrevented = true;
      },
      stopImmediatePropagation() {
        this.stopped = true;
      },
      ...overrides,
    };
    listeners.get(type)?.(value);
    return value;
  }
  return {
    actions,
    options,
    event,
    dispose,
    errors,
    listeners,
    get pastes() {
      return pastes;
    },
    get focuses() {
      return focuses;
    },
  };
}

test("无动作不读剪贴板，保留远端鼠标按下/松开，禁用默认菜单及系统中键粘贴", () => {
  const f = fixture();
  for (const button of [1, 2]) {
    assert.equal(f.event("mousedown", button).stopped, false);
    assert.equal(f.event("mouseup", button).stopped, false);
    assert.equal(f.event("auxclick", button).defaultPrevented, true);
  }
  assert.equal(f.event("mousedown", 1).defaultPrevented, true);
  assert.equal(f.event("contextmenu", 2).stopped, true);
  assert.equal(f.pastes, 0);
  assert.equal(f.focuses, 0);
});

test("右键与中键独立选择粘贴，一个手势只读取一次且不传给远端鼠标协议", () => {
  for (const button of [1, 2]) {
    const f = fixture();
    f.actions[button] = "paste";
    const down = f.event("mousedown", button);
    assert.equal(down.defaultPrevented, true);
    assert.equal(down.stopped, true);
    assert.equal(f.focuses, 1);
    assert.equal(f.pastes, 1);
    assert.equal(f.event("mouseup", button).stopped, true);
    f.event("contextmenu", button);
    f.event("auxclick", button);
    assert.equal(f.pastes, 1);
    f.event("mousedown", button === 1 ? 2 : 1);
    assert.equal(f.pastes, 1);
  }
});

test("动作读取最新设置，主键不粘贴，macOS Control+单击按右键处理", () => {
  const f = fixture();
  f.actions[2] = "paste";
  f.event("mousedown", 0);
  assert.equal(f.pastes, 0);
  f.event("mousedown", 2);
  f.actions[2] = "none";
  f.event("mousedown", 2);
  assert.equal(f.pastes, 1);
  f.options.isMac = true;
  f.actions[2] = "paste";
  assert.equal(f.event("mousedown", 0, { ctrlKey: true }).stopped, true);
  assert.equal(f.pastes, 2);
});

test("断线、关闭或弹层门禁拒绝粘贴，读取失败由调用方展示，销毁移除监听", async () => {
  const f = fixture();
  f.actions[1] = "paste";
  f.options.allowed = false;
  assert.equal(f.event("mousedown", 1).stopped, true);
  assert.equal(f.pastes, 0);
  assert.equal(f.focuses, 0);
  f.options.allowed = true;
  const error = new Error("剪贴板不可用");
  f.options.paste = async () => {
    throw error;
  };
  f.event("mousedown", 1);
  await Promise.resolve();
  assert.deepEqual(f.errors, [error]);
  f.dispose();
  assert.equal(f.listeners.size, 0);
});

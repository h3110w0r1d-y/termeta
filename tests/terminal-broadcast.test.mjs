import assert from "node:assert/strict";
import test from "node:test";
import {
  createInputRouter,
  cursorInput,
  isMouseReport,
} from "../src/modules/terminal/inputRouter.ts";
import { observeTerminalInput } from "../src/modules/terminal/inputOrigin.ts";

const text = (data, overrides = {}) => ({
  data,
  binary: false,
  userInput: true,
  wheel: false,
  ...overrides,
});
function setup(targets = ["a", "b", "c", "a"]) {
  const sent = [];
  const wheels = [];
  const router = createInputRouter({
    targets: () => targets,
    send: (id, bytes) => sent.push({ id, bytes: [...bytes] }),
    applicationCursor: (id) => id === "b",
    wheel(id, event) {
      wheels.push(id);
      // 模拟目标xterm同步生成滚轮输入和自身滚轮回调，不应递归广播。
      router.wheel(id, event);
      router.data(id, text("\x1b[A", { wheel: true }));
    },
  });
  return { router, sent, wheels };
}
const decoded = (sent) =>
  sent.map(({ id, bytes }) => [
    id,
    new TextDecoder().decode(Uint8Array.from(bytes)),
  ]);

test("键盘、控制键和IME只向每个成员发送一次", () => {
  for (const input of [
    "echo test\r",
    "中文输入",
    "\x03",
    "\x04",
    "\t",
    "\x1bb",
  ]) {
    const { router, sent } = setup();
    router.data("a", text(input));
    assert.deepEqual(decoded(sent), [
      ["a", input],
      ["b", input],
      ["c", input],
    ]);
  }
});

test("自动应答、焦点报告、点击/释放/移动和二进制鼠标数据仅回原终端", () => {
  for (const packet of [
    text("\x1b[12;34R", { userInput: false }),
    text("\x1b[I", { userInput: false }),
    text("\x1b[O", { userInput: false }),
    text("\x1b[<0;4;5M"),
    text("\x1b[<0;4;5m"),
    text("\x1b[<32;4;5M"),
    text("\x1b[M !!", { binary: true }),
    text("\x1b[32;4;5M"),
  ]) {
    const { router, sent } = setup();
    router.data("a", packet);
    assert.deepEqual(
      sent.map((item) => item.id),
      ["a"],
    );
  }
  assert.ok(isMouseReport("\x1b[<64;100;200M"));
  assert.equal(isMouseReport("echo mouse"), false);
});

test("滚轮独立分发且不重复广播其方向键/鼠标编码", () => {
  const { router, sent, wheels } = setup();
  router.wheel("a", {});
  router.data("a", text("\x1b[A", { wheel: true }));
  assert.deepEqual(wheels, ["b", "c"]);
  assert.deepEqual(
    sent.map((item) => item.id),
    ["b", "c", "a"],
  );
});

test("方向键按目标应用光标模式编码，不改动普通文本/其他序列", () => {
  const { router, sent } = setup();
  router.data("a", text("\x1b[A"));
  assert.deepEqual(decoded(sent), [
    ["a", "\x1b[A"],
    ["b", "\x1bOA"],
    ["c", "\x1b[A"],
  ]);
  assert.equal(cursorInput("\x1bOF", false), "\x1b[F");
  assert.equal(cursorInput("\x1b[1;5D", true), "\x1b[1;5D");
  assert.equal(cursorInput("hello A", true), "hello A");
});

test("粘贴展开目标后只本地发送，嵌套保护和异常清理不污染后续输入", () => {
  const { router, sent } = setup();
  router.local("b", () =>
    router.local("b", () => router.data("b", text("paste\r"))),
  );
  assert.deepEqual(decoded(sent), [["b", "paste\r"]]);
  assert.throws(() =>
    router.local("a", () => {
      throw new Error("test");
    }),
  );
  sent.length = 0;
  router.data("a", text("x"));
  assert.deepEqual(
    sent.map((item) => item.id),
    ["a", "b", "c"],
  );
});

test("目标快照由外部选择器限定，无成员时仍只发原终端", () => {
  const { router, sent } = setup([]);
  router.data("a", text("x"));
  assert.deepEqual(decoded(sent), [["a", "x"]]);
});

function fakeTerminal(withMarker = true) {
  let marker;
  let data;
  let binary;
  const events = new Map();
  const disposed = [];
  const term = {
    _core: {
      coreService: withMarker
        ? {
            onUserInput(listener) {
              marker = listener;
              return {
                dispose() {
                  marker = undefined;
                  disposed.push("marker");
                },
              };
            },
          }
        : {},
    },
    onData(listener) {
      data = listener;
      return {
        dispose() {
          data = undefined;
          disposed.push("data");
        },
      };
    },
    onBinary(listener) {
      binary = listener;
      return {
        dispose() {
          binary = undefined;
          disposed.push("binary");
        },
      };
    },
    element: {
      addEventListener(name, listener) {
        events.set(name, listener);
      },
      removeEventListener(name) {
        events.delete(name);
      },
    },
  };
  return {
    term,
    events,
    disposed,
    send(value, user = false) {
      if (user) marker?.();
      data?.(value);
    },
    binary(value) {
      binary?.(value);
    },
  };
}

test("来源桥接同步消费用户标记，应答不继承标记，滚轮作用域及时清理", async () => {
  const fake = fakeTerminal();
  const received = [];
  const observer = observeTerminalInput(fake.term, (packet) =>
    received.push(packet),
  );
  assert.equal(observer.supported, true);
  fake.send("中", true);
  fake.send("\x1b[12;3R");
  assert.deepEqual(
    received.map((packet) => packet.userInput),
    [true, false],
  );
  fake.events.get("wheel")();
  fake.send("\x1b[A", true);
  assert.equal(received.at(-1).wheel, true);
  await Promise.resolve();
  fake.send("x", true);
  assert.equal(received.at(-1).wheel, false);
  observer.dispose();
  assert.deepEqual(fake.disposed, ["marker", "data", "binary"]);
  assert.equal(fake.events.size, 0);
});

test("缺少内部来源事件时显式不可广播，单终端通道仍工作", () => {
  const fake = fakeTerminal(false);
  const received = [];
  const observer = observeTerminalInput(fake.term, (packet) =>
    received.push(packet),
  );
  assert.equal(observer.supported, false);
  fake.send("x");
  fake.binary("\x1b[M !!");
  assert.equal(received[0].userInput, false);
  assert.equal(received[1].binary, true);
  observer.dispose();
});

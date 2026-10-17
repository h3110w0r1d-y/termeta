import assert from "node:assert/strict";
import test from "node:test";
import {
  createInputRouter,
  cursorInput,
  isMouseReport,
  broadcastTargets,
  decodeWheelInput,
} from "../src/modules/terminal/inputRouter.ts";
import {
  observeTerminalInput,
  sendFinalWheelInput,
} from "../src/modules/terminal/inputOrigin.ts";

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
    wheel(id, final) {
      wheels.push(id);
      // 模拟目标编码最终动作，只回目标会话，不能把收到的广播再次扩散。
      const sequence = "\x1b[" + (final.direction === "up" ? "A" : "B");
      router.data(id, text(cursorInput(sequence, id === "b")));
    },
  });
  return { router, sent, wheels };
}
const decoded = (sent) =>
  sent.map(({ id, bytes }) => [
    id,
    new TextDecoder().decode(Uint8Array.from(bytes)),
  ]);

test("命令窗格隔离键盘、控制键、粘贴和滚轮，直接操作仍然有效", () => {
  const members = [
    { terminalId: "a", sshCommand: false },
    { terminalId: "b", sshCommand: true },
    { terminalId: "c", sshCommand: false },
  ];
  const sent = [],
    wheels = [];
  const router = createInputRouter({
    targets: (id) => broadcastTargets(id, members),
    send: (id, bytes) => sent.push([id, new TextDecoder().decode(bytes)]),
    applicationCursor: () => false,
    wheel: (id) => wheels.push(id),
  });
  router.data("a", text("echo hello\r"));
  assert.deepEqual(sent.splice(0), [
    ["a", "echo hello\r"],
    ["c", "echo hello\r"],
  ]);
  for (const data of ["q", "\x03", "paste\r"]) {
    router.data("b", text(data));
    assert.deepEqual(sent.splice(0), [["b", data]]);
  }
  router.data("a", text("\x1b[A", { wheel: true }));
  assert.deepEqual(wheels.splice(0), ["c"]);
  router.data("b", text("\x1b[A", { wheel: true }));
  assert.deepEqual(wheels, []);
  assert.deepEqual(broadcastTargets("b", members), ["b"]);
  members[1].sshCommand = false;
  assert.deepEqual(broadcastTargets("a", members), ["a", "b", "c"]);
});

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

test("只广播源端最终滚轮动作，各目标编码一次且不再扩散", () => {
  const { router, sent, wheels } = setup();
  router.data("a", text("\x1b[A", { wheel: true }));
  assert.deepEqual(wheels, ["b", "c"]);
  assert.deepEqual(decoded(sent), [
    ["a", "\x1b[A"],
    ["b", "\x1bOA"],
    ["c", "\x1b[A"],
  ]);
});

test("最终滚轮动作识别方向键、SGR和二进制上报，排除点击/拖动/应答", () => {
  assert.equal(decodeWheelInput("\x1bOB", false).direction, "down");
  assert.deepEqual(decodeWheelInput("\x1b[<85;4;5M", false), {
    direction: "down",
    mouse: true,
    ctrl: true,
    alt: false,
    shift: true,
  });
  assert.equal(
    decodeWheelInput("\x1b[M" + String.fromCharCode(96, 40, 41), true)
      .direction,
    "up",
  );
  for (const data of [
    "\x1b[<0;4;5M",
    "\x1b[<32;4;5M",
    "\x1b[<64;4;5m",
    "\x1b[<66;4;5M",
    "\x1b[12;34R",
    "text",
  ]) {
    assert.equal(decodeWheelInput(data, false), undefined);
    const { router, sent, wheels } = setup();
    router.data("a", text(data, { wheel: true }));
    assert.deepEqual(
      sent.map((item) => item.id),
      ["a"],
    );
    assert.deepEqual(wheels, []);
  }
});

test("二进制滚轮可广播，普通二进制鼠标和自动应答保持本地", () => {
  const { router, sent, wheels } = setup();
  router.data(
    "a",
    text("\x1b[M" + String.fromCharCode(97, 40, 41), {
      wheel: true,
      binary: true,
      userInput: false,
    }),
  );
  assert.deepEqual(wheels, ["b", "c"]);
  assert.deepEqual(
    sent.map((item) => item.id),
    ["a", "b", "c"],
  );
  sent.length = 0;
  wheels.length = 0;
  router.data("a", text("\x1b[A", { wheel: true, userInput: false }));
  assert.deepEqual(
    sent.map((item) => item.id),
    ["a"],
  );
  assert.deepEqual(wheels, []);
});

test("当前组未聚焦成员可作为滚轮来源，键盘仍受焦点约束，命令及组外来源隔离", () => {
  const members = [
    { terminalId: "a" },
    { terminalId: "b" },
    { terminalId: "monitor", sshCommand: true },
  ];
  const sent = [],
    kinds = [];
  let router;
  router = createInputRouter({
    targets(id, kind) {
      kinds.push(kind);
      return broadcastTargets(id, members, "a", kind);
    },
    send: (id) => sent.push(id),
    applicationCursor: () => false,
    wheel: (id) => router.data(id, text("\x1b[A")),
  });
  router.data("b", text("\x1b[A", { wheel: true }));
  assert.deepEqual(sent.splice(0), ["b", "a"]);
  assert.deepEqual(kinds.splice(0), ["wheel"]);
  router.data("b", text("x"));
  assert.deepEqual(sent.splice(0), ["b"]);
  assert.deepEqual(kinds.splice(0), ["keyboard"]);
  for (const id of ["monitor", "hidden-group"]) {
    router.data(id, text("\x1b[A", { wheel: true }));
    assert.deepEqual(sent.splice(0), [id]);
  }
});

test("最终鼠标动作直接调用目标编码器并映射坐标，不调用滚轮累积器", () => {
  const reports = [],
    points = [],
    inputs = [];
  const source = {
    element: {
      querySelector: () => ({
        getBoundingClientRect: () => ({
          left: 10,
          top: 20,
          width: 200,
          height: 100,
        }),
      }),
    },
  };
  const screen = {
    getBoundingClientRect: () => ({
      left: 300,
      top: 400,
      width: 100,
      height: 200,
    }),
  };
  const target = {
    modes: { mouseTrackingMode: "vt200", applicationCursorKeysMode: true },
    buffer: { active: { type: "alternate" } },
    element: { querySelector: () => screen },
    input: (data) => inputs.push(data),
    _core: {
      _mouseService: {
        getMouseReportCoords(point, element) {
          assert.equal(element, screen);
          points.push(point);
          return { col: 4, row: 5, x: 50, y: 100 };
        },
      },
      coreMouseService: {
        consumeWheelEvent() {
          throw new Error("目标不得再次累积滚轮");
        },
        triggerMouseEvent(report) {
          reports.push(report);
          return true;
        },
      },
    },
  };
  const final = decodeWheelInput("\x1b[<81;4;5M", false);
  assert.equal(
    sendFinalWheelInput(target, final, source, { clientX: 110, clientY: 70 }),
    true,
  );
  assert.deepEqual(points, [{ clientX: 350, clientY: 500 }]);
  assert.deepEqual(reports, [
    {
      col: 4,
      row: 5,
      x: 50,
      y: 100,
      button: 4,
      action: 1,
      ctrl: true,
      alt: false,
      shift: false,
    },
  ]);
  assert.deepEqual(inputs, []);
  target.modes.mouseTrackingMode = "none";
  sendFinalWheelInput(target, final, source);
  assert.deepEqual(inputs, ["\x1bOB"]);
  target.buffer.active.type = "normal";
  assert.equal(sendFinalWheelInput(target, final, source), false);
  assert.deepEqual(inputs, ["\x1bOB"]);
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
      coreMouseService: { triggerMouseEvent: () => true },
      _mouseService: {
        getMouseReportCoords: () => ({ col: 0, row: 0, x: 0, y: 0 }),
      },
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
  const event = {};
  assert.equal(fake.events.size, 0);
  // 模拟原生事件在捕获与 xterm 监听器之间的微任务检查点。
  await Promise.resolve();
  fake.send("\x1b[12;3R");
  assert.equal(received.at(-1).wheel, false);
  observer.markWheelInput(event);
  fake.send("\x1b[A", true);
  assert.equal(received.at(-1).wheel, true);
  assert.equal(received.at(-1).wheelEvent, event);
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

test("缺少目标鼠标编码能力时拒绝广播，直接输入和订阅清理仍可用", () => {
  const fake = fakeTerminal();
  delete fake.term._core.coreMouseService;
  const received = [];
  const observer = observeTerminalInput(fake.term, (packet) =>
    received.push(packet),
  );
  assert.equal(observer.supported, false);
  fake.send("x", true);
  assert.equal(received[0].userInput, true);
  observer.dispose();
  assert.deepEqual(fake.disposed, ["marker", "data", "binary"]);
});

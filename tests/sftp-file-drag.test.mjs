import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import test from "node:test";
import { createSftpFileDrag } from "../src/modules/files/sftpFileDrag.ts";
import { sftpPathArguments } from "../src/modules/files/sftpPathArguments.ts";

test("多路径保持顺序，用空格分隔且不添加回车", () => {
  assert.equal(
    sftpPathArguments("/home/user/", ["one", "two"]),
    "/home/user/one /home/user/two",
  );
  assert.equal(sftpPathArguments("/", ["tmp"]), "/tmp");
  assert.equal(sftpPathArguments("/home/a b", ["one"]), "'/home/a b/one'");
});

test(
  "空格、引号和 Shell 特殊字符作为原始参数传入，不发生展开",
  { skip: process.platform === "win32" },
  () => {
    const names = [
      "a b",
      "a'b",
      'a"b',
      "$(printf expanded)",
      "`printf expanded`",
      "x; printf expanded",
      "*.txt",
      "[ab]",
      "!history",
      "#note",
      "a&b|c>d<e",
      "文 件",
      "-option",
    ];
    const text = sftpPathArguments("/home/a'b", names);
    assert.equal(typeof text, "string");
    assert.doesNotMatch(text, /[\r\n]/u);
    // 只在用例明确获准执行时启动本机 POSIX Shell，核对真实参数解析结果。
    const output = execFileSync(
      "/bin/sh",
      ["-c", `set -- ${text}; printf '%s\\0' "$@"`],
      { encoding: "utf8" },
    );
    assert.deepEqual(
      output.split("\0").slice(0, -1),
      names.map((name) => `/home/a'b/${name}`),
    );
  },
);

test("非法条目和超限路径整批拒绝，不产生部分粘贴", () => {
  for (const name of [
    "",
    ".",
    "..",
    "a/b",
    "a\\b",
    "a\ncommand",
    "a\rcommand",
    "a\x1bcommand",
    "a\0b",
  ])
    assert.equal(sftpPathArguments("/tmp", ["valid", name]), undefined);
  for (const directory of [".", "relative", "/tmp\ncommand", "/a\\b"])
    assert.equal(sftpPathArguments(directory, ["one"]), undefined);
  assert.equal(sftpPathArguments("/tmp", []), undefined);
  assert.equal(sftpPathArguments("/tmp", ["文".repeat(22000)]), undefined);
});

function fixture() {
  const state = { valid: true, feedback: undefined };
  const pasted = [],
    native = [];
  const selection = {
    sessionId: "session",
    path: "/tmp",
    names: ["file", "folder"],
    directories: ["folder"],
    nativeAllowed: true,
  };
  const drag = createSftpFileDrag({
    valid: () => state.valid,
    insideWindow: ({ x, y }) => x >= 0 && x < 500 && y >= 0 && y < 300,
    overTerminal: ({ x, y }) => x >= 0 && x < 250 && y >= 0 && y < 300,
    update: (value) => {
      state.feedback = value;
    },
    paste: (value) => pasted.push(value),
    native: (value) => native.push(value),
  });
  return { state, pasted, native, selection, drag };
}

test("内部落点只插入一次路径，不触发原生下载", () => {
  const { drag, selection, pasted, native, state } = fixture();
  drag.start(1, { x: 350, y: 80 }, selection);
  drag.move(1, { x: 100, y: 80 }, 1);
  assert.equal(state.feedback.overTerminal, true);
  assert.deepEqual(native, []);
  drag.end(1, { x: 100, y: 80 });
  drag.end(1, { x: 100, y: 80 });
  assert.deepEqual(pasted, [selection]);
  assert.deepEqual(native, []);
  assert.equal(state.feedback, undefined);
});

test("仅离开窗口才交给原生拖出，之后松键不重复处理", () => {
  const { drag, selection, pasted, native, state } = fixture();
  drag.start(1, { x: 350, y: 80 }, selection);
  drag.move(1, { x: 370, y: 80 }, 1);
  assert.deepEqual(native, []);
  drag.move(1, { x: 510, y: 80 }, 1);
  drag.end(1, { x: 100, y: 80 });
  assert.deepEqual(native, [selection]);
  assert.deepEqual(pasted, []);
  assert.equal(state.feedback, undefined);
});

for (const reason of [
  "cancel",
  "sessionChanged",
  "released",
  "click",
  "wrongPointer",
  "otherArea",
]) {
  test(`手势取消或无效落点不产生输入/传输：${reason}`, () => {
    const { drag, selection, state, pasted, native } = fixture();
    drag.start(1, { x: 350, y: 80 }, selection);
    if (reason === "cancel") drag.cancel();
    if (reason === "sessionChanged") state.valid = false;
    if (reason !== "click")
      drag.move(
        reason === "wrongPointer" ? 2 : 1,
        { x: 100, y: 80 },
        reason === "released" ? 0 : 1,
      );
    drag.end(1, { x: reason === "otherArea" ? 350 : 100, y: 80 });
    assert.deepEqual(pasted, []);
    assert.deepEqual(native, []);
    assert.equal(state.feedback, undefined);
  });
}

test("不支持原生拖出的条目仍可插入路径", () => {
  const { drag, selection, pasted, native } = fixture();
  selection.nativeAllowed = false;
  drag.start(1, { x: 350, y: 80 }, selection);
  drag.move(1, { x: 100, y: 80 }, 1);
  drag.end(1, { x: 100, y: 80 });
  assert.deepEqual(pasted, [selection]);
  drag.start(2, { x: 350, y: 80 }, selection);
  drag.move(2, { x: 510, y: 80 }, 1);
  assert.deepEqual(native, []);
});

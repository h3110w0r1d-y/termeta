import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { runInNewContext } from "node:vm";
import ts from "typescript";
import { createSessionControl } from "../src/modules/sessions/control.ts";

function fixture() {
  const timers = new Map();
  const writes = [];
  const source = readFileSync(
    new URL("../src/modules/terminal/runtime/stream.ts", import.meta.url),
    "utf8",
  );
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.CommonJS,
    },
  });
  const dependencies = {
    "@/i18n": { t: (key) => key },
    "@/services/desktop/client": {
      desktop: { ack: async () => {} },
      errorMessage: (error) => error.message,
    },
    "@/modules/sessions/control": { createSessionControl },
  };
  const exports = {};
  runInNewContext(outputText, {
    exports,
    ArrayBuffer,
    Uint8Array,
    DataView,
    crypto: { randomUUID: () => "request-a" },
    setTimeout(callback, delay) {
      const token = {};
      timers.set(token, { callback, delay });
      return token;
    },
    clearTimeout(token) {
      timers.delete(token);
    },
    require(id) {
      assert.ok(Object.hasOwn(dependencies, id), `未提供依赖：${id}`);
      return dependencies[id];
    },
  });
  const stream = exports.createStream();
  stream.control.sessionId = "session-a";
  const entry = {
    stream,
    term: {
      write(data, callback) {
        writes.push({ data, callback });
      },
    },
  };
  return { ...exports, stream, entry, timers, writes };
}

test("终态先于最后数据帧时，等待解析及 xterm 栅栏再结束排空", async () => {
  const { stream, entry, timers, writes, drain, receive } = fixture();
  stream.control.ended = true;
  stream.finalOffset = 3n;
  let complete = false;
  const draining = drain(entry, stream).then(() => {
    complete = true;
  });
  assert.equal(writes.length, 0);
  assert.equal(timers.size, 1);
  const frame = new ArrayBuffer(27);
  const view = new DataView(frame);
  view.setUint8(0, 1);
  view.setUint32(4, 1, true);
  view.setBigUint64(8, 3n, true);
  view.setUint32(16, 3, true);
  receive(entry, {}, stream, frame);
  assert.equal(stream.pending, 3);
  assert.equal(complete, false);
  writes.shift().callback();
  assert.equal(stream.pending, 0);
  assert.equal(stream.consumed, 3n);
  assert.equal(complete, false);
  assert.equal(writes.length, 1);
  writes.shift().callback();
  await draining;
  assert.equal(complete, true);
  assert.equal(stream.progress.size, 0);
  // 只剩 ACK 合并计时器，排空过程没有周期轮询任务。
  assert.deepEqual(
    [...timers.values()].map((timer) => timer.delay),
    [16],
  );
});

test("排空超时移除等待者并保留流，迟到解析后可再次安全归档", async () => {
  const { stream, entry, timers, writes, drain } = fixture();
  stream.finalOffset = 1n;
  const draining = drain(entry, stream);
  const rejected = assert.rejects(draining, /outputPending/);
  [...timers.values()].find((timer) => timer.delay === 3000).callback();
  await rejected;
  assert.equal(stream.progress.size, 0);
  assert.equal(entry.disposed, undefined);
  stream.consumed = 1n;
  const retry = drain(entry, stream);
  writes.shift().callback();
  await retry;
  assert.equal(stream.progress.size, 0);
});

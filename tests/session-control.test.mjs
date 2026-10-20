import assert from "node:assert/strict";
import test from "node:test";
import {
  acceptSessionEvent,
  acceptSessionSnapshot,
  acceptsSessionChallenge,
  createSessionControl,
} from "../src/modules/sessions/control.ts";

const snapshot = (overrides = {}) => ({
  protocolVersion: 1,
  requestId: "request-a",
  sessionId: "session-a",
  terminalId: "terminal-a",
  kind: "local",
  sshCommand: false,
  capabilities: {
    resize: true,
    pty: true,
    binaryOutput: true,
    serialControl: false,
  },
  title: "本地终端",
  size: { cols: 80, rows: 24 },
  state: "created",
  stateRevision: 0,
  ...overrides,
});
const payloads = {
  hostKeyChallenge: {
    challengeId: "host-key-a",
    endpoint: { host: "example.test", port: 22 },
    algorithm: "ssh-ed25519",
    fingerprint: "测试指纹",
    expiresAt: 1000,
  },
  authChallenge: {
    challengeId: "challenge-a",
    name: "认证",
    instructions: "",
    prompts: [],
    expiresAt: 1000,
  },
  stateChanged: { snapshot: snapshot() },
  ended: {
    finalState: "closed",
    reason: "userClosed",
    finalOffset: "0",
    outputComplete: true,
    exitCode: null,
    exitSignal: null,
    error: null,
  },
};
const event = (type, overrides = {}) => ({
  protocolVersion: 1,
  requestId: "request-a",
  sessionId: "session-a",
  eventSeq: 1,
  type,
  ...payloads[type],
  ...overrides,
});

test("命令响应前的挑战先绑定会话，较旧初始快照不回退状态", () => {
  const control = createSessionControl("request-a");
  assert.equal(acceptSessionEvent(control, event("authChallenge")), true);
  assert.equal(control.sessionId, "session-a");
  assert.equal(
    acceptSessionEvent(
      control,
      event("stateChanged", {
        eventSeq: 2,
        snapshot: snapshot({ state: "awaitingAuth", stateRevision: 3 }),
      }),
    ),
    true,
  );
  assert.equal(acceptSessionSnapshot(control, snapshot()), false);
  assert.equal(control.state, "awaitingAuth");
  assert.equal(control.revision, 3);
});

test("终态早于初始响应时不复活，后续挑战和重复结束被忽略", () => {
  const control = createSessionControl("request-a");
  assert.equal(
    acceptSessionEvent(
      control,
      event("ended", {
        finalState: "failed",
      }),
    ),
    true,
  );
  assert.equal(
    acceptSessionSnapshot(control, snapshot({ stateRevision: 9 })),
    false,
  );
  assert.equal(
    acceptSessionEvent(control, event("authChallenge", { eventSeq: 2 })),
    false,
  );
  assert.equal(
    acceptSessionEvent(
      control,
      event("ended", { eventSeq: 3, finalState: "closed" }),
    ),
    false,
  );
  assert.equal(control.state, "failed");
});

test("请求、会话、版本和事件序号共同隔离旧流", () => {
  const control = createSessionControl("request-a");
  assert.equal(
    acceptSessionEvent(
      control,
      event("authChallenge", { requestId: "request-old" }),
    ),
    false,
  );
  assert.equal(control.sessionId, undefined);
  assert.equal(acceptSessionSnapshot(control, snapshot()), true);
  for (const invalid of [
    { sessionId: "session-old" },
    { protocolVersion: 2 },
    { eventSeq: 0 },
    { eventSeq: NaN },
  ])
    assert.equal(
      acceptSessionEvent(control, event("authChallenge", invalid)),
      false,
    );
  assert.equal(
    acceptSessionEvent(control, event("authChallenge", { eventSeq: 4 })),
    true,
  );
  assert.equal(
    acceptSessionEvent(control, event("authChallenge", { eventSeq: 3 })),
    false,
  );
  assert.equal(
    acceptSessionSnapshot(
      control,
      snapshot({ sessionId: "session-old", stateRevision: 5 }),
    ),
    false,
  );
});

test("查询先前进 revision 时仍消费旧状态事件序号，避免旧挑战重放", () => {
  const control = createSessionControl("request-a");
  acceptSessionSnapshot(
    control,
    snapshot({ state: "connected", stateRevision: 8 }),
  );
  assert.equal(
    acceptSessionEvent(
      control,
      event("stateChanged", {
        eventSeq: 10,
        snapshot: snapshot({ state: "authenticating", stateRevision: 2 }),
      }),
    ),
    false,
  );
  assert.equal(
    acceptSessionEvent(control, event("authChallenge", { eventSeq: 9 })),
    false,
  );
  assert.equal(control.state, "connected");
  assert.equal(
    acceptSessionEvent(
      control,
      event("ended", { eventSeq: 11, finalState: "closed" }),
    ),
    true,
  );
});

test("嵌套快照不能借用合法信封改绑其他会话或请求", () => {
  const control = createSessionControl("request-a");
  for (const invalid of [
    { sessionId: "session-other" },
    { requestId: "request-other" },
    { protocolVersion: 2 },
  ])
    assert.equal(
      acceptSessionEvent(
        control,
        event("stateChanged", {
          snapshot: snapshot(invalid),
        }),
      ),
      false,
    );
  assert.equal(control.sessionId, undefined);
  assert.equal(control.eventSeq, 0);
});

test("查询先取得关闭或连接结果时，不重开排队中的旧挑战，仍消费最终输出结束", () => {
  for (const state of [
    "connected",
    "closing",
    "closed",
    "failed",
    "disconnected",
  ]) {
    for (const type of ["authChallenge", "hostKeyChallenge"]) {
      const control = createSessionControl("request-a");
      acceptSessionSnapshot(control, snapshot({ state, stateRevision: 8 }));
      assert.equal(
        acceptSessionEvent(
          control,
          event("stateChanged", {
            eventSeq: 2,
            snapshot: snapshot({ state: "awaitingAuth", stateRevision: 2 }),
          }),
        ),
        false,
      );
      // 旧挑战本来就排在旧等待状态之后，事件序号更高也不能说明它仍有效。
      assert.equal(
        acceptSessionEvent(control, event(type, { eventSeq: 3 })),
        false,
      );
      assert.equal(control.eventSeq, 3);
      assert.equal(control.state, state);
      assert.equal(
        acceptSessionEvent(
          control,
          event("ended", { eventSeq: 4, finalOffset: "128" }),
        ),
        true,
      );
    }
  }
});

test("首挑战保持可用，队列只保留当前等待阶段的提示", () => {
  for (const state of ["created", "connecting"]) {
    for (const type of ["authChallenge", "hostKeyChallenge"]) {
      const control = createSessionControl("request-a");
      acceptSessionSnapshot(control, snapshot({ state }));
      assert.equal(acceptSessionEvent(control, event(type)), true);
    }
  }
  const challengeTypes = ["authChallenge", "hostKeyChallenge"];
  assert.deepEqual(
    challengeTypes.filter((type) =>
      acceptsSessionChallenge("awaitingAuth", type),
    ),
    ["authChallenge"],
  );
  assert.deepEqual(
    challengeTypes.filter((type) =>
      acceptsSessionChallenge("awaitingHostKey", type),
    ),
    ["hostKeyChallenge"],
  );
  for (const state of [
    "authenticating",
    "opening",
    "connected",
    "closing",
    "closed",
    "failed",
    "disconnected",
  ])
    assert.deepEqual(
      challengeTypes.filter((type) => acceptsSessionChallenge(state, type)),
      [],
    );
});

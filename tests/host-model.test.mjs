import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { runInNewContext } from "node:vm";
import ts from "typescript";

// 替换文案加载，直接检查唯一桌面适配器对旧 wire 的转换与拒绝边界。
const { outputText } = ts.transpileModule(
  readFileSync(
    new URL("../src/services/desktop/hosts.ts", import.meta.url),
    "utf8",
  ),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } },
);
const exports = {};
runInNewContext(outputText, {
  exports,
  require(id) {
    assert.equal(id, "@/i18n");
    return { t: (key) => key };
  },
});
const { hostProfileFromDto, hostDraftToDto, hostInventoryFromDto } = exports;
const plain = (value) => JSON.parse(JSON.stringify(value));
const serialConfig = {
  path: "/dev/fixture-only",
  baudRate: 115200,
  charset: "GB18030",
  dataBits: 7,
  stopBits: 2,
  flowControl: "hardware",
  parity: "even",
};
const workspace = {
  root: {
    kind: "split",
    axis: "horizontal",
    ratio: 0.3,
    first: { kind: "connection", profileId: "ssh" },
    second: { kind: "connection", profileId: "serial" },
  },
  focusedIndex: 1,
};
const metadata = {
  id: "saved",
  name: "保存项",
  groupId: "group",
  tags: ["保留标签"],
  icon: "ssh",
  revision: 9,
  sortOrder: 42,
};
function dto(kind, fields = {}) {
  return {
    ...metadata,
    kind,
    address: "",
    port: 0,
    username: "",
    authPlan: [],
    hasPassword: false,
    systemDetectionAttempted: false,
    startup: { type: "shell" },
    serialConfig: null,
    workspace: null,
    ...fields,
  };
}

test("四种保存项保留元数据与有效配置，领域模型不再携带其他协议占位", () => {
  const ssh = dto("ssh", {
    address: "example.test",
    port: 2222,
    username: "operator",
    authPlan: [{ type: "privateKey", credentialId: "credential" }],
    hasPassword: true,
    systemDetectionAttempted: true,
    startup: { type: "exec", command: "docker stats" },
  });
  const cases = [
    [
      ssh,
      {
        ...metadata,
        kind: "ssh",
        address: ssh.address,
        port: ssh.port,
        username: ssh.username,
        authPlan: ssh.authPlan,
        hasPassword: true,
        systemDetectionAttempted: true,
        startup: ssh.startup,
      },
    ],
    [
      dto("telnet", { address: "telnet.test", port: 2323 }),
      {
        ...metadata,
        kind: "telnet",
        address: "telnet.test",
        port: 2323,
      },
    ],
    [
      dto("serial", { serialConfig }),
      { ...metadata, kind: "serial", serialConfig },
    ],
    [
      dto("workspace", { workspace }),
      { ...metadata, kind: "workspace", workspace },
    ],
  ];
  for (const [wire, expected] of cases) {
    const saved = hostProfileFromDto(wire);
    assert.deepEqual(plain(saved), expected);
    if (saved.kind === "workspace") continue;
    const draft = hostDraftToDto(saved);
    assert.equal(draft.id, wire.id);
    assert.equal(draft.name, wire.name);
    assert.deepEqual(plain(draft.tags), wire.tags);
    for (const field of ["address", "port", "username", "authPlan"])
      assert.deepEqual(
        plain(draft[field]),
        wire[field],
        `${saved.kind}.${field}`,
      );
    if (saved.kind === "ssh")
      assert.deepEqual(plain(draft.startup), wire.startup);
    if (saved.kind === "serial")
      assert.deepEqual(plain(draft.serialConfig), serialConfig);
  }
});

test("旧 SSH 缺省启动字段保持缺省，不把命令或认证引用替换为默认值", () => {
  const old = dto("ssh", {
    address: "old.test",
    port: 22,
    username: "user",
    authPlan: [{ type: "password" }],
  });
  delete old.startup;
  const saved = hostProfileFromDto(old);
  assert.equal(saved.startup, undefined);
  const draft = hostDraftToDto(saved);
  assert.equal(draft.startup, undefined);
  assert.deepEqual(plain(draft.authPlan), [{ type: "password" }]);
});

test("缺失必需配置或包含跨协议字段时拒绝，禁止清除后再保存", () => {
  assert.throws(
    () => hostProfileFromDto(dto("serial")),
    /serial.errors.missingConfiguration/,
  );
  assert.throws(
    () => hostProfileFromDto(dto("workspace")),
    /workspaces.errors.missingLayout/,
  );
  for (const broken of [
    dto("unknown"),
    dto("ssh", { serialConfig }),
    dto("ssh", { workspace }),
    dto("telnet", { username: "legacy-user" }),
    dto("telnet", { authPlan: [{ type: "password", credentialId: "saved" }] }),
    dto("telnet", { startup: { type: "exec", command: "must-not-disappear" } }),
    dto("serial", { serialConfig, address: "legacy-host" }),
    dto("serial", { serialConfig, port: 22 }),
    dto("workspace", { workspace, hasPassword: true }),
  ])
    assert.throws(
      () => hostProfileFromDto(broken),
      /hosts.errors.invalidConfiguration/,
    );
});

test("清单在全部保存项完成适配后返回，不过滤损坏配置或工作区引用", () => {
  const groups = [
    { id: "group", parentId: null, name: "分组", sortOrder: 0, revision: 1 },
  ];
  const inventory = hostInventoryFromDto({
    hosts: [dto("workspace", { workspace })],
    groups,
  });
  assert.equal(inventory.groups, groups);
  assert.deepEqual(plain(inventory.hosts[0].workspace), workspace);
  assert.throws(
    () => hostInventoryFromDto({ hosts: [dto("workspace")], groups }),
    /workspaces.errors.missingLayout/,
  );
});

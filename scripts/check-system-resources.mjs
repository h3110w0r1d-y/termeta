// 仅读取静态清单和源码，不导入业务模块、不执行检测脚本或触发编译。
import { readFile, readdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { dirname, resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFile(join(root, path), "utf8");
const manifest = JSON.parse(await read("resources/host-system-icons.json"));
const core = await read("crates/terminal-core/src/system.rs");
const rustIds = [
  ...core
    .split("pub const HOST_SYSTEM_ICONS")[1]
    .split("];")[0]
    .matchAll(/"([a-z_]+)"/g),
].map((match) => match[1]);
assert.deepEqual(
  manifest.map(({ id }) => id),
  rustIds,
  "前后端图标白名单必须一致",
);
assert.equal(new Set(manifest.map(({ id }) => id)).size, manifest.length);
for (const entry of manifest) {
  assert.match(entry.background, /^#[0-9a-f]{6}$/i);
  assert.match(entry.colorVariable, /^--[a-zA-Z-]+$/);
  assert.match(entry.file, /^[a-z_]+\.svg$/);
  const svg = await read(`src/assets/svg/systems/${entry.file}`);
  assert.equal(
    createHash("sha256").update(svg).digest("hex"),
    entry.sha256,
    `${entry.id}素材摘要不一致`,
  );
  assert(
    !/<(?:script|foreignObject)\b|\bon\w+\s*=|(?:href|src)\s*=\s*["'](?!#)/i.test(
      svg,
    ),
    `${entry.id}包含主动内容或外部引用`,
  );
  assert.match(svg, /<svg\b/);
}
assert.deepEqual(
  (await readdir(join(root, "src/assets/svg/systems"))).sort(),
  manifest.map(({ file }) => file).sort(),
);
const scripts = JSON.parse(await read("resources/ssh-os-detection.json"));
assert.deepEqual(Object.keys(scripts), [
  "shell",
  "posix",
  "fish",
  "routeros",
  "windows",
]);
for (const script of Object.values(scripts))
  assert(
    typeof script === "string" && script.length > 0 && script.length < 4096,
  );
assert(
  (await read("crates/terminal-ssh/src/os_detection.rs")).includes(
    'include_str!("../../../resources/ssh-os-detection.json")',
  ),
);
assert(
  (await read("src/modules/settings/SystemDetectionSettings.vue")).includes(
    'import scripts from "../../../resources/ssh-os-detection.json"',
  ),
);
console.log(
  `系统资源静态检查通过：${manifest.length}个图标、5份共用只读命令；未执行远端命令。`,
);

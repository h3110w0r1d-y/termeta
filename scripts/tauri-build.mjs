import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const envPath = resolve(root, ".env");
const environment = { ...process.env };

if (existsSync(envPath)) {
  for (const line of readFileSync(envPath, "utf8").split(/\r?\n/)) {
    const entry = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$/);
    if (!entry || entry[1].startsWith("#")) continue;
    const value = entry[2].replace(/^(['"])(.*)\1$/, "$2");
    if (!(entry[1] in environment)) environment[entry[1]] = value;
  }
}

const command = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
const result = spawnSync(command, ["tauri", "build"], {
  cwd: root,
  env: environment,
  stdio: "inherit",
});

if (result.error) throw result.error;
process.exit(result.status ?? 1);

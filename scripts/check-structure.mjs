// 仅读取源码和清单，不导入业务模块，也不触发编译或运行。
import { readdir, readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
async function walk(directory) {
  const entries = await readdir(join(root, directory), { withFileTypes: true });
  const nested = await Promise.all(
    entries.map((entry) => {
      const path = `${directory}/${entry.name}`;
      return entry.isDirectory() ? walk(path) : [path];
    }),
  );
  return nested.flat();
}
const errors = [];
const large = [];
for (const path of (await walk("src")).filter((path) =>
  /\.(ts|vue)$/.test(path),
)) {
  const text = await readFile(join(root, path), "utf8");
  // 静态数据单独管理，不按业务代码行数提示。
  if (!path.startsWith("src/assets/")) {
    const lines = text.trimEnd().split("\n").length;
    if (lines > 500) large.push(`${path}: ${lines} 行（人工复查职责）`);
  }
  if (path.endsWith(".vue")) {
    // 表单 model 必须直接引用响应式状态，不能在渲染时创建值快照。
    const computedModels = new Set(
      [
        ...text.matchAll(
          /\b(?:const|let)\s+(\w+)\s*=\s*computed(?:<[^;]*?>)?\s*\(/g,
        ),
      ].map((match) => match[1]),
    );
    for (const match of text.matchAll(/<NForm\b[^>]*?:model="([^"]+)"/g)) {
      const model = match[1].trim();
      if (model.startsWith("{") || computedModels.has(model)) {
        errors.push(
          `${path}: NForm model 应绑定共享响应式状态，禁止临时对象或 computed 派生 model`,
        );
      }
    }
  }
  const imports = [
    ...text.matchAll(
      /(?:\bfrom\s*|\bimport\s*\(\s*|\bimport\s*)["']([^"']+)["']/g,
    ),
  ].map((match) => match[1]);
  if (
    !path.startsWith("src/services/desktop/") &&
    imports.some((name) => name.startsWith("@tauri-apps/"))
  ) {
    errors.push(`${path}: 原生 API 必须通过 desktop 门面`);
  }
  if (
    path.startsWith("src/services/commands/") &&
    imports.some((name) => name.includes("modules/"))
  ) {
    errors.push(`${path}: 命令基础设施不得反向依赖业务模块`);
  }
}
const manifest = await readFile(
  join(root, "crates/terminal-core/Cargo.toml"),
  "utf8",
);
if (
  /^\s*(?:tauri(?:-[\w-]+)?|russh(?:-[\w-]+)?|sqlx|serialport|portable-pty|terminal-(?:ssh|local|serial|sftp|storage|security|files))\s*=/m.test(
    manifest,
  )
) {
  errors.push("terminal-core: 不得依赖具体协议、存储或桌面适配器");
}
for (const path of (await walk("src-tauri/src/commands")).filter((path) =>
  path.endsWith(".rs"),
)) {
  const text = await readFile(join(root, path), "utf8");
  if (/use\s+super::(?:files|file_drag|file_workspace)::/.test(text)) {
    errors.push(`${path}: 文件命令共用实现应位于壳内 files 模块`);
  }
}
for (const message of large) console.log(`提示: ${message}`);
for (const message of errors) console.error(`错误: ${message}`);
if (errors.length) process.exitCode = 1;
else console.log("结构静态检查通过；行数提示不代表必须拆分。");

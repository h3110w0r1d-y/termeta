// 仅读取源码和清单，不导入业务模块，也不触发编译或运行。
import { readdir, readFile } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
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
  if (imports.some((name) => name.startsWith("../../"))) {
    errors.push(`${path}: 跨越两层及以上的导入须使用 @/ 或 @resources/ 别名`);
  }
  // 按实际源路径约束共享模块，别名与相对导入遵循同一依赖方向。
  const sourceImports = imports.map((name) =>
    name.startsWith("@/")
      ? `src/${name.slice(2)}`
      : name.startsWith(".")
        ? relative(root, resolve(root, dirname(path), name)).replaceAll(
            "\\",
            "/",
          )
        : name,
  );
  if (
    path.startsWith("src/theme/") &&
    sourceImports.some(
      (name) =>
        name.startsWith("src/modules/") || name.startsWith("src/layout/"),
    )
  ) {
    errors.push(`${path}: 共享主题不得反向依赖业务模块或窗口装配`);
  }
  if (
    path.startsWith("src/modules/sessions/") &&
    sourceImports.some(
      (name) =>
        name.startsWith("src/layout/") ||
        (name.startsWith("src/modules/") &&
          !name.startsWith("src/modules/sessions/")),
    )
  ) {
    errors.push(`${path}: 通用会话模块不得反向依赖布局或具体会话用途`);
  }
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
  /^\s*(?:tauri(?:-[\w-]+)?|russh(?:-[\w-]+)?|sqlx|serialport|portable-pty|terminal-(?:ssh|local|serial|telnet|sftp|storage|security|files))\s*=/m.test(
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
// 检查显式命令清单，不能替代 Rust/TS 序列化契约及运行时 owner 校验。
const namesSource = await readFile(
  join(root, "src-tauri/src/command_names.rs"),
  "utf8",
);
const commandNames = [...namesSource.matchAll(/"([a-z][a-z0-9_]+)"/g)].map(
  (match) => match[1],
);
const commands = new Set(commandNames);
const shellSource = await readFile(join(root, "src-tauri/src/lib.rs"), "utf8");
const handlerSource =
  shellSource.match(/tauri::generate_handler!\[([\s\S]*?)\]/)?.[1] ?? "";
const handlerNames = handlerSource
  .split(",")
  .map((path) => path.trim().split("::").at(-1))
  .filter(Boolean);
const handlers = new Set(handlerNames);
if (
  commands.size !== commandNames.length ||
  handlers.size !== handlerNames.length
) {
  errors.push("IPC: 命令清单或 handler 存在重复项");
}
for (const name of new Set([...commands, ...handlers])) {
  if (!commands.has(name) || !handlers.has(name)) {
    errors.push(`IPC: ${name} 必须同时登记 AppManifest 命令清单与 handler`);
  }
}
for (const path of (await walk("src/services/desktop")).filter((path) =>
  path.endsWith(".ts"),
)) {
  const source = await readFile(join(root, path), "utf8");
  for (const match of source.matchAll(
    /\binvoke(?:<[^;]*?>)?\s*\(\s*["']([^"']+)["']/g,
  )) {
    if (!match[1].includes(":") && !commands.has(match[1])) {
      errors.push(`${path}: invoke 命令 ${match[1]} 未登记`);
    }
  }
}
// 自定义权限组与自动生成的 allow-command 均计入，避免只检查新增命令的特例。
const permissionGroups = new Map();
for (const path of (await walk("src-tauri/permissions")).filter((path) =>
  path.endsWith(".toml"),
)) {
  const source = await readFile(join(root, path), "utf8");
  for (const section of source.split("[[permission]]").slice(1)) {
    const identifier = section.match(/identifier\s*=\s*"([^"]+)"/)?.[1];
    const allowed =
      section.match(/commands\.allow\s*=\s*\[([\s\S]*?)\]/)?.[1] ?? "";
    if (identifier)
      permissionGroups.set(
        identifier,
        [...allowed.matchAll(/"([^"]+)"/g)].map((match) => match[1]),
      );
  }
}
const exposed = new Set();
for (const path of (await walk("src-tauri/capabilities")).filter((path) =>
  path.endsWith(".json"),
)) {
  const capability = JSON.parse(await readFile(join(root, path), "utf8"));
  for (const permission of capability.permissions ?? []) {
    if (typeof permission !== "string") continue;
    for (const name of permissionGroups.get(permission) ?? [])
      exposed.add(name);
    if (permission.startsWith("allow-")) {
      const name = permission.slice(6).replaceAll("-", "_");
      if (commands.has(name)) exposed.add(name);
    }
  }
}
for (const name of commands) {
  if (!exposed.has(name)) errors.push(`IPC: ${name} 缺少 capability 授权入口`);
}
for (const message of large) console.log(`提示: ${message}`);
for (const message of errors) console.error(`错误: ${message}`);
if (errors.length) process.exitCode = 1;
else console.log("结构静态检查通过；行数提示不代表必须拆分。");

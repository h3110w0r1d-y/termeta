import { readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import ts from "typescript";

// 仅静态解析用户提供的数组字面量，不执行解包程序或压缩代码。
const [input, output] = process.argv.slice(2);
if (!input || !output)
  throw new Error(
    "用法：node scripts/extract-terminal-palettes.mjs <输入JS> <输出JSON>",
  );
const source = await readFile(input, "utf8");
const marker = "colorSchemesArray=";
const start = source.indexOf(marker);
if (start < 0) throw new Error("未找到配色数组");
const begin = start + marker.length;
let depth = 0,
  quote = "",
  escaped = false,
  end = -1;
for (let i = begin; i < source.length; i++) {
  const c = source[i];
  if (quote) {
    if (escaped) escaped = false;
    else if (c === "\\") escaped = true;
    else if (c === quote) quote = "";
  } else if (c === '"' || c === "'") quote = c;
  else if (c === "[") depth++;
  else if (c === "]" && --depth === 0) {
    end = i + 1;
    break;
  }
}
if (end < 0) throw new Error("配色数组边界无效");
const literal = source.slice(begin, end);
const ast = ts.createSourceFile(
  "palette.js",
  `const data=${literal}`,
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.JS,
);
if (ast.parseDiagnostics.length) throw new Error("数组解析失败");
function read(node) {
  if (ts.isStringLiteral(node)) return node.text;
  if (ts.isArrayLiteralExpression(node)) return node.elements.map(read);
  if (ts.isObjectLiteralExpression(node)) {
    const result = Object.create(null);
    for (const property of node.properties) {
      if (
        !ts.isPropertyAssignment(property) ||
        !(ts.isIdentifier(property.name) || ts.isStringLiteral(property.name))
      )
        throw new Error("仅接受静态对象属性");
      const key = property.name.text;
      if (Object.hasOwn(result, key)) throw new Error(`重复字段：${key}`);
      result[key] = read(property.initializer);
    }
    return result;
  }
  throw new Error(`拒绝非数据表达式：${ts.SyntaxKind[node.kind]}`);
}
const declaration = ast.statements[0]?.declarationList?.declarations[0];
if (!declaration?.initializer) throw new Error("缺少数组声明");
const palettes = read(declaration.initializer);
if (!Array.isArray(palettes) || !palettes.some((p) => p.name === "Nord Dark"))
  throw new Error("缺少 Nord Dark");
for (const p of palettes) {
  if (
    typeof p.name !== "string" ||
    !Array.isArray(p.colorPaletteOverrides) ||
    p.colorPaletteOverrides.length !== 16
  )
    throw new Error("配色字段不完整");
  for (const color of [
    ...p.colorPaletteOverrides,
    p.backgroundColor,
    p.foregroundColor,
    p.cursorColor,
    p.selectionColor,
    p.app?.terminalUIColor,
  ])
    if (typeof color !== "string" || !/^#[\da-f]{6}([\da-f]{2})?$/i.test(color))
      throw new Error(`非法颜色：${p.name}`);
}
const ids = new Set();
const schemes = palettes.map((p) => {
  const id = p.name
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  if (!id || ids.has(id)) throw new Error(`重复或无效方案 ID：${p.name}`);
  ids.add(id);
  return {
    schemaVersion: 1,
    id,
    name: p.name,
    source: "builtin",
    colors: {
      foreground: p.foregroundColor,
      background: p.backgroundColor,
      ui: p.app.terminalUIColor,
      cursor: p.cursorColor,
      // 原数据未给出光标反色与失焦选区，分别沿用背景与原选区色。
      cursorAccent: p.backgroundColor,
      selectionBackground: p.selectionColor,
      selectionInactiveBackground: p.selectionColor,
      ansi: p.colorPaletteOverrides,
    },
  };
});
await writeFile(output, JSON.stringify(schemes, null, 2) + "\n");
console.log(
  JSON.stringify(
    {
      count: palettes.length,
      literalBytes: Buffer.byteLength(literal),
      literalSha256: createHash("sha256").update(literal).digest("hex"),
      fields: [...new Set(palettes.flatMap(Object.keys))],
      names: palettes.map((p) => p.name),
    },
    null,
    2,
  ),
);

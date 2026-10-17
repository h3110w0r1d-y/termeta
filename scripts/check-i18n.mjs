// 仅读取与解析源码，检查词条覆盖及插值契约，不导入或运行应用。
import { readdir, readFile } from "node:fs/promises";
import { resolve } from "node:path";
import ts from "typescript";
import { parse } from "vue/compiler-sfc";
const root = resolve(import.meta.dirname, "..");
const errors = [];
const messages = new Map();
const used = new Set();
const placeholders = (value) =>
  [...value.matchAll(/\{(\w+)\}/g)]
    .map((m) => m[1])
    .sort()
    .join(",");
for (const name of await readdir(resolve(root, "src/i18n/messages"))) {
  const source = ts.createSourceFile(
    name,
    await readFile(resolve(root, "src/i18n/messages", name), "utf8"),
    ts.ScriptTarget.Latest,
    true,
  );
  function visit(node) {
    if (ts.isPropertyAssignment(node) && ts.isStringLiteral(node.initializer)) {
      const key = node.name.text;
      const value = node.initializer.text;
      if (!value.trim()) errors.push(`${name}: 空译文 ${key}`);
      if (placeholders(key) !== placeholders(value))
        errors.push(`${name}: 插值参数不一致 ${key}`);
      if (messages.has(key) && messages.get(key) !== value)
        errors.push(`${name}: 相同中文词条存在不同译文 ${key}`);
      messages.set(key, value);
    }
    ts.forEachChild(node, visit);
  }
  visit(source);
}
async function walk(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "assets" && entry.name !== "i18n") await walk(path);
      continue;
    }
    if (!/\.(ts|vue)$/.test(path)) continue;
    const text = await readFile(path, "utf8");
    const check = (code) => {
      const source = ts.createSourceFile(
        path,
        code,
        ts.ScriptTarget.Latest,
        true,
      );
      function visit(node) {
        if (
          ts.isCallExpression(node) &&
          ts.isIdentifier(node.expression) &&
          node.expression.text === "t"
        ) {
          const first = node.arguments[0];
          if (
            first &&
            (ts.isStringLiteral(first) ||
              ts.isNoSubstitutionTemplateLiteral(first))
          ) {
            used.add(first.text);
            if (!messages.has(first.text))
              errors.push(`${path}: 缺少英文词条 ${first.text}`);
          }
        }
        ts.forEachChild(node, visit);
      }
      visit(source);
    };
    if (path.endsWith(".vue")) {
      const { descriptor, errors: parseErrors } = parse(text, {
        filename: path,
      });
      for (const error of parseErrors) errors.push(`${path}: ${String(error)}`);
      if (descriptor.script) check(descriptor.script.content);
      if (descriptor.scriptSetup) check(descriptor.scriptSetup.content);
      function template(node) {
        if (node.type === 2 && /\p{Script=Han}/u.test(node.content))
          errors.push(`${path}: 未翻译的模板文字 ${node.content.trim()}`);
        if (node.type === 5) check(node.content.content);
        for (const prop of node.props ?? []) {
          if (prop.type === 7 && prop.exp) check(prop.exp.content);
          if (
            prop.type === 6 &&
            prop.value &&
            /\p{Script=Han}/u.test(prop.value.content)
          )
            errors.push(
              `${path}: 未翻译的属性 ${prop.name}=${prop.value.content}`,
            );
        }
        for (const child of node.children ?? []) template(child);
      }
      if (descriptor.template?.ast) template(descriptor.template.ast);
    } else check(text);
  }
}
await walk(resolve(root, "src"));
if (errors.length) {
  console.error(errors.join("\n"));
  process.exitCode = 1;
} else
  console.log(
    `多语言静态检查通过：${messages.size} 个词条，${used.size} 个直接引用；模板文字与插值参数一致。`,
  );

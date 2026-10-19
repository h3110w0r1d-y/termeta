// 仅读取与解析源码，检查词条覆盖及插值契约，不导入或运行应用。
import { readdir, readFile } from "node:fs/promises";
import { resolve } from "node:path";
import ts from "typescript";
import { parse } from "vue/compiler-sfc";
const root = resolve(import.meta.dirname, "..");
const errors = [];
const used = new Set();
const keyPattern = /^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)+$/;
const parametersOf = (value) =>
  [
    ...new Set([...value.matchAll(/\{(\w+)\}/g)].map((match) => match[1])),
  ].sort();
const sameParameters = (first, second) =>
  parametersOf(first).join(",") === parametersOf(second).join(",");

function sourceFile(path, text, checkSyntax = true) {
  const source = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true);
  for (const diagnostic of checkSyntax ? source.parseDiagnostics : [])
    errors.push(
      `${path}: ${ts.flattenDiagnosticMessageText(diagnostic.messageText, " ")}`,
    );
  return source;
}

async function readMessages(locale) {
  const messages = new Map();
  const directory = resolve(root, "src/i18n/messages", locale);
  for (const name of await readdir(directory)) {
    if (!/\.(ts|json)$/.test(name) || name === "index.ts") continue;
    const path = resolve(directory, name);
    const contents = await readFile(path, "utf8");
    if (name.endsWith(".json")) {
      try {
        JSON.parse(contents);
      } catch {
        errors.push(`${path}: 词典必须使用严格 JSON 格式`);
      }
    }
    const source = sourceFile(path, contents);
    const assignment = name.endsWith(".json")
      ? source.statements.find(ts.isExpressionStatement)
      : source.statements.find(ts.isExportAssignment);
    let expression = assignment?.expression;
    if (expression && ts.isAsExpression(expression))
      expression = expression.expression;
    if (!expression || !ts.isObjectLiteralExpression(expression)) {
      errors.push(`${path}: 词典必须为 JSON 或默认导出的纯文本对象`);
      continue;
    }
    for (const property of expression.properties) {
      if (
        !ts.isPropertyAssignment(property) ||
        !ts.isStringLiteral(property.name) ||
        !ts.isStringLiteral(property.initializer)
      ) {
        errors.push(`${path}: 词条必须使用字符串 key 与纯文本译文`);
        continue;
      }
      const key = property.name.text;
      const value = property.initializer.text;
      if (!keyPattern.test(key)) errors.push(`${path}: 无效的语义 key ${key}`);
      if (!key.startsWith(`${name.replace(/\.(ts|json)$/, "")}.`))
        errors.push(`${path}: key 与模块命名空间不一致 ${key}`);
      if (!value.trim()) errors.push(`${path}: 空译文 ${key}`);
      if (messages.has(key)) errors.push(`${path}: 重复 key ${key}`);
      messages.set(key, value);
    }
  }
  if (!messages.size) errors.push(`${locale}: 词典不能为空`);
  return messages;
}

const locales = ["zh-CN", "en-US", "zh-TW", "ja-JP", "ko-KR"];
const dictionaries = new Map(
  await Promise.all(
    locales.map(async (locale) => [locale, await readMessages(locale)]),
  ),
);
const chinese = dictionaries.get("zh-CN");
for (const [locale, dictionary] of dictionaries) {
  for (const [key, value] of chinese) {
    if (!dictionary.has(key)) errors.push(`${locale}: 缺少词条 ${key}`);
    else if (!sameParameters(value, dictionary.get(key)))
      errors.push(`${locale}: 插值参数不一致 ${key}`);
  }
  for (const key of dictionary.keys())
    if (!chinese.has(key)) errors.push(`${locale}: 多余词条 ${key}`);
}

// 原生菜单与前端共享 JSON 资源，Rust 固定 key 同样需要覆盖和纯文本约束。
const nativeSource = (
  await Promise.all(
    ["items.rs", "system_items.rs"].map((file) =>
      readFile(resolve(root, "src-tauri/src/macos_menu", file), "utf8"),
    ),
  )
).join("\n");
const nativeKeys = new Set(
  [...nativeSource.matchAll(/"(nativeMenu\.[a-zA-Z0-9.]+)"/g)].map(
    (match) => match[1],
  ),
);
for (const key of nativeKeys) {
  used.add(key);
  if ([...dictionaries.values()].some((dictionary) => !dictionary.has(key)))
    errors.push(`原生菜单: 缺少词条 ${key}`);
}
for (const key of chinese.keys()) {
  if (!key.startsWith("nativeMenu.")) continue;
  if (!nativeKeys.has(key)) errors.push(`原生菜单: 未使用的词条 ${key}`);
  for (const [locale, messages] of dictionaries) {
    const value = messages.get(key);
    if (value && /[{}\u0000-\u001f\u007f]/u.test(value))
      errors.push(`${locale}: 原生菜单只允许无插值的单行文案 ${key}`);
  }
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
      // Vue 指令包含 v-for、插槽参数等语法片段，由 Vue 类型检查负责语法校验。
      const source = sourceFile(path, code, false);
      function visit(node) {
        if (
          ts.isCallExpression(node) &&
          ts.isIdentifier(node.expression) &&
          node.expression.text === "t"
        ) {
          const first = node.arguments[0];
          if (first && ts.isStringLiteralLike(first)) {
            const key = first.text;
            used.add(key);
            if (!chinese.has(key))
              errors.push(`${path}: 未定义的语义 key ${key}`);
            else {
              const expected = parametersOf(chinese.get(key));
              const parameters = node.arguments[1];
              if (expected.length && !parameters)
                errors.push(
                  `${path}: 缺少插值参数 ${key}: ${expected.join(",")}`,
                );
              if (parameters && ts.isObjectLiteralExpression(parameters)) {
                // 展开对象的字段由类型检查验证；显式字段可以直接核对拼写。
                const hasSpread = parameters.properties.some(
                  ts.isSpreadAssignment,
                );
                const supplied = parameters.properties
                  .filter(
                    (property) =>
                      property.name &&
                      !ts.isComputedPropertyName(property.name),
                  )
                  .map((property) => property.name.text);
                for (const name of expected)
                  if (!hasSpread && !supplied.includes(name))
                    errors.push(`${path}: 缺少插值参数 ${key}: ${name}`);
                for (const name of supplied)
                  if (!expected.includes(name))
                    errors.push(`${path}: 多余插值参数 ${key}: ${name}`);
              }
            }
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
    `多语言静态检查通过：${chinese.size} 个语义 key，${used.size} 个直接引用；${locales.length} 种语言词条、命名空间、模板文字与插值参数一致。`,
  );

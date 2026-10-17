import { t } from "../../i18n";
// 在 JSON.parse 前检查重复键和嵌套深度，防止不同解析器对导入配置作出不同解释。
export function parseJson(text: string): unknown {
  let at = 0;
  function space() {
    while (/\s/.test(text[at] ?? "") && at < text.length) at++;
  }
  function string(): string {
    const start = at++;
    while (at < text.length) {
      if (text[at] === "\\") {
        at += 2;
        continue;
      }
      if (text[at++] === '"')
        return JSON.parse(text.slice(start, at)) as string;
    }
    throw new Error(t("json.errors.unterminatedString"));
  }
  function value(depth: number): void {
    if (depth > 20) throw new Error(t("json.errors.excessiveDepth"));
    space();
    const char = text[at];
    if (char === '"') {
      string();
      return;
    }
    if (char === "{" || char === "[") {
      at++;
      space();
      const end = char === "{" ? "}" : "]",
        keys = new Set<string>();
      let count = 0;
      if (text[at] === end) {
        at++;
        return;
      }
      for (;;) {
        if (++count > 1000) throw new Error(t("json.errors.tooManyEntries"));
        space();
        if (char === "{") {
          if (text[at] !== '"')
            throw new Error(t("json.errors.invalidProperty"));
          const key = string();
          if (keys.has(key))
            throw new Error(t("json.errors.duplicateProperty", { name: key }));
          keys.add(key);
          space();
          if (text[at++] !== ":")
            throw new Error(t("json.errors.missingColon"));
        }
        value(depth + 1);
        space();
        if (text[at] === end) {
          at++;
          return;
        }
        if (text[at++] !== ",")
          throw new Error(t("json.errors.invalidCollection"));
      }
    }
    const start = at;
    while (at < text.length && !/[\s,\]}]/.test(text[at]!)) at++;
    if (start === at) throw new Error(t("json.errors.invalidValue"));
    JSON.parse(text.slice(start, at));
  }
  value(0);
  space();
  if (at !== text.length) throw new Error(t("json.errors.trailingContent"));
  return JSON.parse(text);
}

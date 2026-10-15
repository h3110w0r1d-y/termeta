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
    throw new Error("JSON 字符串未结束");
  }
  function value(depth: number): void {
    if (depth > 20) throw new Error("JSON 嵌套超过 20 层");
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
        if (++count > 1000) throw new Error("JSON 集合条目过多");
        space();
        if (char === "{") {
          if (text[at] !== '"') throw new Error("JSON 属性名无效");
          const key = string();
          if (keys.has(key)) throw new Error(`JSON 存在重复属性：${key}`);
          keys.add(key);
          space();
          if (text[at++] !== ":") throw new Error("JSON 缺少冒号");
        }
        value(depth + 1);
        space();
        if (text[at] === end) {
          at++;
          return;
        }
        if (text[at++] !== ",") throw new Error("JSON 集合格式错误");
      }
    }
    const start = at;
    while (at < text.length && !/[\s,\]}]/.test(text[at]!)) at++;
    if (start === at) throw new Error("JSON 值无效");
    JSON.parse(text.slice(start, at));
  }
  value(0);
  space();
  if (at !== text.length) throw new Error("JSON 含多余内容");
  return JSON.parse(text);
}

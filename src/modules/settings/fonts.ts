// 字体名称是持久化契约；资源和版本来源见 public/fonts/manifest.json。
export const terminalFonts = [
  "DejaVu Sans Mono",
  "Droid Sans Mono",
  "Ubuntu Mono",
  "Anonymous Pro",
  "Inconsolata",
  "Fira Mono",
  "Source Code Pro",
  "PT Mono",
  "Cascadia Code",
  "Fira Code",
  "JetBrains Mono",
  "Meslo",
] as const;
export const defaultTerminalFont = "DejaVu Sans Mono";
export const terminalFontOptions = terminalFonts.map((name) => ({
  label: name,
  value: name,
}));
export function normalizeTerminalFont(name: string): string {
  return terminalFonts.some((font) => font === name)
    ? name
    : defaultTerminalFont;
}
export function terminalFontFamily(name: string): string {
  // 独立别名只引用随应用分发的字体，不受系统同名字体版本影响。
  return `"Termeta ${normalizeTerminalFont(name)}", monospace`;
}
const loads = new Map<string, Promise<void>>();
export function loadTerminalFont(name: string, size: number): Promise<void> {
  const family = terminalFontFamily(name);
  const key = `${size}px ${family}`;
  const existing = loads.get(key);
  if (existing) return existing;
  // 等常规/粗体及斜体完成加载后再让 xterm 测量字宽，避免回退字体造成列宽错误。
  const loading = Promise.all(
    ["normal 400", "normal 700", "italic 400", "italic 700"].map((style) =>
      document.fonts.load(`${style} ${key}`),
    ),
  ).then(() => {});
  loads.set(key, loading);
  void loading.catch(() => loads.delete(key));
  return loading;
}

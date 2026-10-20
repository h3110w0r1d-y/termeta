import type { ITheme, Terminal } from "@xterm/xterm";
import { SerializeAddon } from "@xterm/addon-serialize";
import { t } from "@/i18n";

const prefix = "Termeta terminal snapshot v1\n";
const ansiKeys = [
  "black",
  "red",
  "green",
  "yellow",
  "blue",
  "magenta",
  "cyan",
  "white",
  "brightBlack",
  "brightRed",
  "brightGreen",
  "brightYellow",
  "brightBlue",
  "brightMagenta",
  "brightCyan",
  "brightWhite",
] as const;

export interface TerminalLogSnapshot {
  cols: number;
  rows: number;
  foreground: string;
  background: string;
  ansi: string[];
  normal: string;
  screen?: string;
}

function isColor(value: unknown): value is string {
  return (
    typeof value === "string" && /^#[\da-f]{6}(?:[\da-f]{2})?$/i.test(value)
  );
}

function invalidSnapshot(): Error {
  return new Error(t("sessionLogs.invalidSnapshot"));
}

function isSerializedBuffer(value: string): boolean {
  // 只接受固定 SerializeAddon 输出的 SGR、定位、擦除和备用屏切换，拒绝 OSC/链接等动作。
  const text = value.replace(/\x1b\[(?:[\d;]*[ABCDHXm]|\?1049h)/g, "");
  return !/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f-\x9f]/.test(text);
}

export function captureTerminalLog(terminal: Terminal): string {
  // 固定 xterm 6 的配色服务同时包含 OSC 动态调色；options.theme 不包含这些变化。
  // 访问集中在此适配器，结构缺失时保留原终端，避免静默以错误颜色归档。
  const colors = (
    terminal as Terminal & {
      _core?: {
        _themeService?: {
          colors?: {
            foreground: { css: string };
            background: { css: string };
            ansi: { css: string }[];
          };
        };
      };
    }
  )._core?._themeService?.colors;
  if (!colors) throw invalidSnapshot();
  const addon = new SerializeAddon();
  terminal.loadAddon(addon);
  try {
    const normal = addon.serialize({
      excludeModes: true,
      excludeAltBuffer: true,
    });
    const snapshot: TerminalLogSnapshot = {
      cols: terminal.cols,
      rows: terminal.rows,
      foreground: colors.foreground.css,
      background: colors.background.css,
      ansi: colors.ansi.slice(0, 256).map((color) => color.css),
      normal,
    };
    if (terminal.buffer.active.type === "alternate")
      snapshot.screen = addon.serialize({ excludeModes: true });
    // 存储版本封装，不改变数据库结构；旧日志继续按纯文本读取。
    const content = prefix + JSON.stringify(snapshot);
    readTerminalLogSnapshot(content);
    return content;
  } finally {
    addon.dispose();
  }
}

export function readTerminalLogSnapshot(
  content: string,
): TerminalLogSnapshot | undefined {
  if (!content.startsWith(prefix)) return undefined;
  let value: Partial<TerminalLogSnapshot>;
  try {
    value = JSON.parse(content.slice(prefix.length));
  } catch {
    throw invalidSnapshot();
  }
  if (
    !value ||
    typeof value !== "object" ||
    !Number.isInteger(value.cols) ||
    value.cols! < 1 ||
    value.cols! > 1000 ||
    !Number.isInteger(value.rows) ||
    value.rows! < 1 ||
    value.rows! > 1000 ||
    !isColor(value.foreground) ||
    !isColor(value.background) ||
    !Array.isArray(value.ansi) ||
    value.ansi.length !== 256 ||
    !value.ansi.every(isColor) ||
    typeof value.normal !== "string" ||
    !isSerializedBuffer(value.normal) ||
    (value.screen !== undefined &&
      (typeof value.screen !== "string" || !isSerializedBuffer(value.screen)))
  )
    throw invalidSnapshot();
  return value as TerminalLogSnapshot;
}

export function snapshotTheme(snapshot: TerminalLogSnapshot): ITheme {
  const theme: ITheme = {
    foreground: snapshot.foreground,
    background: snapshot.background,
    extendedAnsi: snapshot.ansi.slice(16),
  };
  ansiKeys.forEach((key, index) => {
    theme[key] = snapshot.ansi[index];
  });
  return theme;
}

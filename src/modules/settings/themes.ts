import { t } from "../../i18n";
import { desktop } from "../../services/desktop/client";
import { parseJson } from "../../services/data/json";
import { clone } from "../../services/data/clone";
import type { ITheme } from "@xterm/xterm";
import type { Scheme } from "./model";
import palettes from "../../../resources/terminal-palettes.json";
// 名称和终端色值由用户提供的配色数组静态提取，映射及摘要见 resources 中的来源记录。
export const builtinSchemes: Scheme[] = palettes.map((palette): Scheme => ({
  ...palette,
  schemaVersion: 1,
  source: "builtin",
}));
export const defaultSchemeId = "pro";
export const defaultScheme = builtinSchemes.find(
  (s) => s.id === defaultSchemeId,
)!;
export function resolveScheme(id: string, custom: Scheme[]) {
  return (
    [...builtinSchemes, ...custom].find((s) => s.id === id) ?? defaultScheme
  );
}
export function toXtermTheme(scheme: Scheme): ITheme {
  const c = scheme.colors;
  const keys = [
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
  ];
  const theme: ITheme = {
    foreground: c.foreground,
    background: c.background,
    cursor: c.cursor,
    cursorAccent: c.cursorAccent,
    selectionBackground: c.selectionBackground,
    selectionForeground: c.selectionForeground,
    selectionInactiveBackground: c.selectionInactiveBackground,
    scrollbarSliderBackground: c.scrollbar?.normal,
    scrollbarSliderHoverBackground: c.scrollbar?.hover,
    scrollbarSliderActiveBackground: c.scrollbar?.active,
  };
  keys.forEach((key, i) => Object.assign(theme, { [key]: c.ansi[i] }));
  if (c.indexedOverrides) {
    const extended = new Array<string>(240);
    for (let n = 16; n < 256; n++) {
      const i = n - 16;
      const rgb =
        n < 232
          ? [Math.floor(i / 36), Math.floor(i / 6) % 6, i % 6].map((v) =>
              v === 0 ? 0 : 55 + 40 * v,
            )
          : [8 + 10 * (n - 232), 8 + 10 * (n - 232), 8 + 10 * (n - 232)];
      extended[i] =
        c.indexedOverrides[String(n)] ??
        `#${rgb.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
    }
    theme.extendedAnsi = extended;
  }
  return theme;
}
const colorPattern = /^#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?$/;
export function parseScheme(text: string): Scheme {
  if (new TextEncoder().encode(text).length > 1024 * 1024)
    throw new Error(t("themes.validation.fileSize"));
  const value: unknown = parseJson(text);
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error(t("themes.validation.singleObject"));
  const raw = value as Record<string, unknown>;
  if (raw.schemaVersion !== undefined && raw.schemaVersion !== 1)
    throw new Error(t("themes.validation.version"));
  const base = clone(defaultScheme);
  let colors: unknown = raw.colors;
  if (!colors) {
    const converted = toXtermTheme(base) as Record<string, unknown>;
    for (const [k, v] of Object.entries(raw))
      if (k in converted && typeof v === "string") converted[k] = v;
    colors = {
      ...base.colors,
      ...converted,
      ansi: [
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
      ].map((k) => converted[k]),
    };
  }
  const c = colors as Scheme["colors"];
  for (const k of [
    "foreground",
    "background",
    "cursor",
    "cursorAccent",
    "selectionBackground",
    "selectionInactiveBackground",
  ] as const)
    if (typeof c[k] !== "string" || !colorPattern.test(c[k]))
      throw new Error(t("themes.validation.colorField", { name: k }));
  if (c.background.length !== 7)
    throw new Error(t("themes.validation.background"));
  if (
    !Array.isArray(c.ansi) ||
    c.ansi.length !== 16 ||
    c.ansi.some((v) => typeof v !== "string" || !/^#[0-9a-fA-F]{6}$/.test(v))
  )
    throw new Error(t("themes.validation.ansi"));
  if (c.selectionForeground && !colorPattern.test(c.selectionForeground))
    throw new Error(t("themes.validation.selectionForeground"));
  for (const [k, v] of Object.entries(c.indexedOverrides ?? {}))
    if (
      !/^\d+$/.test(k) ||
      Number(k) < 16 ||
      Number(k) > 255 ||
      typeof v !== "string" ||
      !colorPattern.test(v)
    )
      throw new Error(t("themes.validation.indexedColors"));
  for (const v of Object.values(c.scrollbar ?? {}))
    if (typeof v !== "string" || !colorPattern.test(v))
      throw new Error(t("themes.validation.scrollbar"));
  return {
    schemaVersion: 1,
    id: crypto.randomUUID(),
    name: String(raw.name ?? t("themes.importName")).slice(0, 80),
    source: "imported",
    colors: {
      foreground: c.foreground,
      background: c.background,
      cursor: c.cursor,
      cursorAccent: c.cursorAccent,
      selectionBackground: c.selectionBackground,
      selectionForeground: c.selectionForeground,
      selectionInactiveBackground: c.selectionInactiveBackground,

      ansi: [...c.ansi],
      indexedOverrides: c.indexedOverrides,
      scrollbar: c.scrollbar,
    },
  };
}
// 由原生保存对话框授予输出路径，前端不构造任意文件写入授权。
export async function downloadJson(
  name: string,
  value: unknown,
): Promise<boolean> {
  return desktop.exportJson(name, value);
}

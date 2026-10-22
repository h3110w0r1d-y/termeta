import { t } from "@/i18n";
import { parseJson } from "@/services/data/json";
import { clone } from "@/services/data/clone";
import type { Scheme, SchemePreferences } from "./types";
import type { OpenTarget } from "@/services/desktop/types";
import { ansiColorKeys, toXtermTheme } from "./xterm";
import palettes from "@resources/terminal-palettes.json";
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
/** 合并工作区与终端渲染共用同一配色优先级，不依赖 xterm 实例。 */
export function resolveTerminalScheme(
  terminal: { schemeId?: string; target: OpenTarget },
  settings: SchemePreferences,
): Scheme {
  return resolveScheme(
    terminal.schemeId ??
      (terminal.target.source === "saved"
        ? settings.hostSchemes[terminal.target.profileId]
        : undefined) ??
      settings.defaultSchemeId,
    settings.customSchemes,
  );
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
      ui: undefined,
      ...converted,
      ansi: ansiColorKeys.map((key) => converted[key]),
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
  if (
    c.ui !== undefined &&
    c.ui !== "" &&
    (typeof c.ui !== "string" || !colorPattern.test(c.ui))
  )
    throw new Error(t("themes.validation.colorField", { name: "ui" }));
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
      ui: c.ui || undefined,
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

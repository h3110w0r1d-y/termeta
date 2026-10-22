import type { ITheme } from "@xterm/xterm";
import type { Scheme } from "./types";

export const ansiColorKeys = [
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

export function toXtermTheme(scheme: Scheme): ITheme {
  const c = scheme.colors;
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
  ansiColorKeys.forEach((key, i) => Object.assign(theme, { [key]: c.ansi[i] }));
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

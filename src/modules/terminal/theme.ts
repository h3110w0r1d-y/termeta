import type { Terminal } from "@xterm/xterm";
import type { Scheme } from "@/theme/types";
import { parseColor, rgba } from "@/theme/colors";
import { toXtermTheme } from "@/theme/xterm";

function selectionOverlayColor(hex: string): string {
  const [red, green, blue, alpha] = parseColor(hex);
  // 与 xterm 保持一致：不透明选色按 30% 透明度显示，避免完全遮住正文。
  return rgba([red, green, blue, alpha === 1 ? 0.3 : alpha]);
}

/** 为终端及预览统一应用主题；选区几何、鼠标和复制仍由 xterm 管理。 */
export function applyTerminalTheme(
  terminal: Terminal,
  host: HTMLElement,
  scheme: Scheme,
): void {
  const theme = toXtermTheme(scheme);
  const overlay = !scheme.colors.selectionForeground;
  host.classList.toggle("terminal-selection-overlay", overlay);
  if (overlay) {
    host.style.setProperty(
      "--terminal-selection-active",
      selectionOverlayColor(scheme.colors.selectionBackground),
    );
    host.style.setProperty(
      "--terminal-selection-inactive",
      selectionOverlayColor(scheme.colors.selectionInactiveBackground),
    );
    // DOM 渲染器会给选中文字另画一层不透明底色。先让该底色回到终端背景，
    // 再由 CSS 将原选区色覆盖文字，避免背景被重复混合；不改变持久化的配色。
    theme.selectionBackground = "#00000000";
    theme.selectionInactiveBackground = "#00000000";
  } else {
    // 显式选区文字色优先，回退到 xterm 原生渲染，不额外蒙色。
    host.style.removeProperty("--terminal-selection-active");
    host.style.removeProperty("--terminal-selection-inactive");
  }
  terminal.options.theme = theme;
}

import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { terminalFontFamily, loadTerminalFont } from "../settings/fonts";
import { useSettings } from "../settings/store";
import { snapshotTheme, type TerminalLogSnapshot } from "./snapshot";

/** 回看实例没有会话、输入路由、链接插件或 IPC，仅还原经过校验的快照。 */
export function mountLogTerminal(
  host: HTMLElement,
  snapshot: TerminalLogSnapshot,
  screen: boolean,
) {
  const settings = useSettings().value;
  const terminal = new Terminal({
    cols: snapshot.cols,
    rows: snapshot.rows,
    scrollback: 51000,
    disableStdin: true,
    cursorBlink: false,
    drawBoldTextInBrightColors: false,
    fontFamily: terminalFontFamily(settings.fontFamily),
    fontSize: settings.fontSize,
    theme: snapshotTheme(snapshot),
    linkHandler: { activate() {} },
    screenReaderMode: true,
  });
  let disposed = false;
  const fit = new FitAddon();
  terminal.loadAddon(fit);
  terminal.parser.registerOscHandler(52, () => true);
  const resize = () => {
    if (disposed || screen || !host.clientHeight) return;
    const dimensions = fit.proposeDimensions();
    // 固定原始列宽；只调整可见行数，避免回看窗口重排并改变日志位置与色块。
    if (dimensions)
      terminal.resize(snapshot.cols, Math.min(1000, dimensions.rows));
  };
  const observer = new ResizeObserver(resize);
  const ready = loadTerminalFont(settings.fontFamily, settings.fontSize).then(
    () => {
      if (disposed) return;
      // 字体就绪后再创建宿主，确保快照列宽使用随包字体的实际字宽。
      terminal.open(host);
      return new Promise<void>((resolve) => {
        terminal.write(screen ? snapshot.screen! : snapshot.normal, () => {
          if (disposed) return resolve();
          terminal.write("\x1b[?25l", () => {
            if (!disposed) {
              resize();
              terminal.scrollToTop();
              observer.observe(host);
            }
            resolve();
          });
        });
      });
    },
  );
  return {
    ready,
    dispose() {
      disposed = true;
      observer.disconnect();
      terminal.dispose();
      host.replaceChildren();
    },
  };
}

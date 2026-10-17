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
    scrollback: screen ? 0 : 51000,
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
  const surface = screen ? document.createElement("div") : host;
  if (screen) {
    host.classList.add("log-screen-viewport");
    host.tabIndex = 0;
    surface.className = "log-screen-surface";
    host.append(surface);
  }
  const fit = new FitAddon();
  terminal.loadAddon(fit);
  terminal.parser.registerOscHandler(52, () => true);
  const resize = () => {
    if (disposed) return;
    if (screen) {
      // 固定备用屏的原始行列；用实际渲染尺寸撑开画布，交给独立视口双向滚动。
      const bounds = terminal.element
        ?.querySelector(".xterm-screen")
        ?.getBoundingClientRect();
      if (bounds?.width && bounds.height) {
        surface.style.width = `${Math.ceil(bounds.width)}px`;
        surface.style.height = `${Math.ceil(bounds.height)}px`;
      }
      return;
    }
    if (!host.clientWidth || !host.clientHeight) return;
    const dimensions = fit.proposeDimensions();
    // 先按原始尺寸还原，再随容器同时调整行列，让 xterm 重排历史并在内部滚动。
    if (
      dimensions &&
      Number.isFinite(dimensions.cols) &&
      Number.isFinite(dimensions.rows)
    )
      terminal.resize(
        Math.min(1000, dimensions.cols),
        Math.min(1000, dimensions.rows),
      );
  };
  const scrollScreen = (event: WheelEvent) => {
    if (event.ctrlKey) return;
    // 在捕获阶段接管，避免备用缓冲将滚轮转成方向键，或内部滚动控件吞掉事件。
    const lineHeight = surface.clientHeight / snapshot.rows;
    const scale =
      event.deltaMode === WheelEvent.DOM_DELTA_LINE
        ? lineHeight
        : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
          ? host.clientHeight
          : 1;
    const horizontal = event.shiftKey && !event.deltaX;
    host.scrollBy({
      left: (horizontal ? event.deltaY : event.deltaX) * scale,
      top: horizontal ? 0 : event.deltaY * scale,
      behavior: "instant",
    });
    event.preventDefault();
    event.stopPropagation();
  };
  if (screen)
    host.addEventListener("wheel", scrollScreen, {
      capture: true,
      passive: false,
    });
  const observer = new ResizeObserver(resize);
  const ready = loadTerminalFont(settings.fontFamily, settings.fontSize).then(
    () => {
      if (disposed) return;
      // 字体就绪后再创建宿主，确保快照列宽使用随包字体的实际字宽。
      terminal.open(surface);
      return new Promise<void>((resolve) => {
        terminal.write(screen ? snapshot.screen! : snapshot.normal, () => {
          if (disposed) return resolve();
          terminal.write("\x1b[?25l", () => {
            if (!disposed) {
              resize();
              terminal.scrollToTop();
              const observed = screen
                ? terminal.element?.querySelector(".xterm-screen")
                : host;
              if (observed) observer.observe(observed);
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
      if (screen) host.removeEventListener("wheel", scrollScreen, true);
      terminal.dispose();
      host.replaceChildren();
    },
  };
}

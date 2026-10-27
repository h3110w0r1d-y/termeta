import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { t } from "@/i18n";
import { desktop, errorMessage } from "@/services/desktop/client";
import { commands, isMac, modifierMatches } from "@/services/commands/registry";
import type { SessionLogDraft } from "@/services/desktop/types";
import { useHosts } from "@/modules/hosts/store";
import { useSettings } from "@/modules/settings/store";
import { loadTerminalFont, terminalFontFamily } from "@/modules/settings/fonts";
import { resolveTerminalScheme } from "@/theme/schemes";
import {
  useWorkbenchStore,
  type TerminalItem,
} from "@/modules/workbench/store";
import { isSessionTerminal } from "@/modules/sessions/control";
import { updateActivityTitle, updateShellActivity } from "../activity";
import { updateWorkingDirectory } from "../workingDirectory";
import { observeTerminalInput, type TerminalInput } from "../inputOrigin";
import { bindTerminalMouseActions } from "../mouseActions";
import { applyTerminalTheme } from "../theme";
import type { Entry } from "./entry";

export function settings(entry: Entry, terminal: TerminalItem) {
  const s = useSettings().value;
  applyTerminalTheme(
    entry.term,
    entry.host,
    resolveTerminalScheme(terminal, s),
  );
  const family = terminalFontFamily(s.fontFamily);
  const request = `${s.fontSize}px ${family}`;
  if (entry.fontRequest !== request) {
    entry.fontRequest = request;
    void loadTerminalFont(s.fontFamily, s.fontSize)
      .then(() => {
        // 迟到的加载不能覆盖较新的选择，也不能访问已关闭的终端。
        if (entry.disposed || entry.fontRequest !== request) return;
        entry.term.options.fontFamily = family;
        fit(entry, terminal);
      })
      .catch(() => {
        if (entry.disposed || entry.fontRequest !== request) return;
        entry.fontRequest = undefined;
        terminal.error = t("terminal.errors.fontLoad");
      });
  }
  entry.term.options.fontSize = s.fontSize;
  entry.term.options.scrollback = s.scrollback;
}
export function fit(entry: Entry, terminal: TerminalItem) {
  if (entry.host.clientWidth < 10 || entry.host.clientHeight < 10) return;
  entry.fit.fit();
  const size = {
    cols: Math.min(1000, entry.term.cols),
    rows: Math.min(1000, entry.term.rows),
  };
  terminal.size = size;
  const key = `${size.cols}:${size.rows}`;
  if (
    entry.canResize &&
    entry.lastSize !== key &&
    terminal.sessionId &&
    terminal.state === "connected"
  ) {
    const stream = entry.stream;
    entry.lastSize = key;
    void desktop.resize(terminal.sessionId, size).catch((error) => {
      if (entry.disposed || entry.stream !== stream || entry.lastSize !== key)
        return;
      entry.lastSize = "";
      terminal.error = errorMessage(error);
    });
  }
}
function connectionLog(
  terminal: TerminalItem,
): Omit<SessionLogDraft, "content"> {
  const target = terminal.target;
  const profile =
    target.source === "saved"
      ? useHosts().hosts.find((item) => item.id === target.profileId)
      : undefined;
  const config =
    target.source === "ephemeral" || target.source === "telnet"
      ? target.config
      : profile?.kind === "ssh" || profile?.kind === "telnet"
        ? profile
        : undefined;
  const protocol = terminal.kind === "forwarding" ? "ssh" : terminal.kind;
  return {
    id: terminal.terminalId,
    protocol,
    name: profile?.name ?? terminal.baseTitle,
    username:
      protocol === "ssh" && config && "username" in config
        ? config.username
        : null,
    host:
      protocol === "serial"
        ? (terminal.serialConfig?.path ?? null)
        : protocol === "local"
          ? null
          : (config?.address ?? null),
    port:
      protocol === "ssh" || protocol === "telnet"
        ? (config?.port ?? null)
        : null,
    startedAt: Date.now(),
  };
}
export function remoteWheel(term: Terminal) {
  return (
    (term.modes.mouseTrackingMode !== "none" &&
      term.modes.mouseTrackingMode !== "x10") ||
    term.buffer.active.type === "alternate"
  );
}

export function createTerminalEntry(
  terminal: TerminalItem,
  host: HTMLElement,
): Entry {
  const term = new Terminal({
    // 粗体仅改变字重，避免普通 ANSI 色被自动替换为高亮色。
    drawBoldTextInBrightColors: false,
    convertEol: false,
    cursorBlink: true,
    allowProposedApi: false,
    scrollback: 5000,
    // 通过 xterm 公开选项让滚动增量按两倍累积，广播仍只跟随源端最终输入。
    scrollSensitivity: 2,
    screenReaderMode: false,
  });
  const fitAddon = new FitAddon(),
    search = new SearchAddon();
  // OSC 52 不能让远端程序读取或改写本机剪贴板。
  term.parser.registerOscHandler(52, () => true);
  term.loadAddon(fitAddon);
  term.loadAddon(search);
  const activate = (e: MouseEvent, url: string) => {
    const binding = useSettings().value.keymap.bindings.find(
      (b) =>
        b.enabled &&
        b.commandId === "terminal.openLink" &&
        b.trigger.type === "pointer",
    );
    if (
      binding &&
      modifierMatches(e, binding.trigger.modifiers) &&
      e.button === 0 &&
      /^https?:\/\//i.test(url)
    ) {
      e.preventDefault();
      // 链接在 mouseup 激活，必须继续冒泡，让 xterm 的 document 监听结束拖选。
      // 下一轮任务再打开浏览器，避免窗口失焦早于鼠标松开事件的清理。
      setTimeout(() => {
        if (entry.disposed) return;
        void desktop.openUrl(url).catch((error) => {
          terminal.error = errorMessage(error);
        });
      }, 0);
    }
  };
  term.loadAddon(new WebLinksAddon(activate));
  term.options.linkHandler = { activate };
  host.dataset.terminalId = terminal.terminalId;
  const entry: Entry = {
    terminal,
    term,
    fit: fitAddon,
    search,
    host,
    observer: new ResizeObserver(() => fit(entry, terminal)),
    log: connectionLog(terminal),
    disposed: false,
    lastSize: "",
    canResize: false,
  };
  return entry;
}

export function mountTerminalView(
  entry: Entry,
  terminal: TerminalItem,
  handlers: {
    data: (packet: TerminalInput) => void;
    paste: (text: string) => Promise<void>;
    reconnect: () => Promise<void>;
    pasteClipboard: () => Promise<void>;
  },
) {
  const { term, host } = entry;
  const canReportDirectory = () =>
    !entry.disposed &&
    !entry.stream?.control.ended &&
    terminal.state === "connected" &&
    terminal.kind === "ssh" &&
    !terminal.sshCommand &&
    term.buffer.active.type === "normal";
  // xterm 统一解析 OSC 0/2；目录兼容从完整原标题提取，避免展示截断影响路径。
  // 订阅随终端实例销毁，标签切换与分屏不重复注册。
  term.onTitleChange((title) => {
    if (!entry.disposed) updateActivityTitle(terminal, title);
    if (canReportDirectory()) updateWorkingDirectory(terminal, title, "title");
  });
  term.parser.registerOscHandler(7, (payload) => {
    if (canReportDirectory()) updateWorkingDirectory(terminal, payload, "osc7");
    return true;
  });
  term.parser.registerOscHandler(1337, (payload) => {
    if (!payload.startsWith("CurrentDir=")) return false;
    if (canReportDirectory())
      updateWorkingDirectory(terminal, payload, "osc1337");
    return true;
  });
  // 只读取Shell执行阶段，不从输出猜命令，也不执行远端控制指令。
  term.parser.registerOscHandler(133, (payload) => {
    if (!entry.disposed) updateShellActivity(terminal, payload);
    return true;
  });
  settings(entry, terminal);
  term.open(host);
  entry.observer.observe(host);
  term.attachCustomKeyEventHandler((e) => {
    if (e.type !== "keydown") return true;
    // 单独按修饰键不应结束输入法组合、清除选区或改变滚屏位置。
    if (["Meta", "Control", "Alt", "Shift"].includes(e.key)) return false;
    if (commands.route(e, useSettings().value.keymap.bindings, true))
      return false;
    if (
      e.key === "Enter" &&
      !e.ctrlKey &&
      !e.metaKey &&
      !e.altKey &&
      !e.shiftKey &&
      !e.repeat &&
      !e.isComposing &&
      e.keyCode !== 229 &&
      !e.defaultPrevented &&
      !e.getModifierState("AltGraph") &&
      !entry.disposed &&
      !entry.closing &&
      !entry.connection &&
      isSessionTerminal(terminal.state) &&
      useWorkbenchStore().activeTerminal?.terminalId === terminal.terminalId &&
      document.activeElement === term.textarea &&
      !document.querySelector("[data-modal-open], [data-ui-popup-open]") &&
      !term.textarea?.closest("[data-shortcut-recorder]")
    ) {
      // 只消费终态下真实按下的回车；不编码为输入，避免发送或广播到新会话。
      e.preventDefault();
      e.stopPropagation();
      void handlers.reconnect().catch((error) => {
        if (!entry.disposed) terminal.error = errorMessage(error);
      });
      return false;
    }
    if (
      isMac &&
      e.altKey &&
      !e.ctrlKey &&
      !e.metaKey &&
      !e.shiftKey &&
      !e.isComposing &&
      !e.defaultPrevented &&
      !e.getModifierState("AltGraph") &&
      !document.querySelector("[data-modal-open], [data-ui-popup-open]") &&
      (e.key === "ArrowLeft" || e.key === "ArrowRight")
    ) {
      // macOS 跳词使用 Shell 常见的 Meta-b/f，避免发送未绑定的修饰方向键序列。
      e.preventDefault();
      e.stopPropagation();
      term.input(e.key === "ArrowLeft" ? "\x1bb" : "\x1bf", true);
      return false;
    }
    return true;
  });
  entry.inputOrigin = observeTerminalInput(term, handlers.data);
  entry.disposeMouseActions = bindTerminalMouseActions(host, {
    isMac,
    action: (button) =>
      button === 2
        ? useSettings().value.terminalRightClickAction
        : useSettings().value.terminalMiddleClickAction,
    canPaste: () =>
      !entry.disposed &&
      !entry.closing &&
      terminal.state === "connected" &&
      useWorkbenchStore().activeTerminal?.terminalId === terminal.terminalId &&
      !document.querySelector("[data-modal-open], [data-ui-popup-open]"),
    focus: () => term.focus(),
    paste: handlers.pasteClipboard,
    onError: (error) => {
      if (!entry.disposed) terminal.error = errorMessage(error);
    },
  });
  term.attachCustomWheelEventHandler((event) => {
    // 只标记即将编码的来源，等实际 onData/onBinary 生成输入后才广播最终动作。
    if (remoteWheel(term) && event.deltaY !== 0)
      entry.inputOrigin?.markWheelInput(event);
    return true;
  });
  host.addEventListener(
    "paste",
    (e) => {
      e.preventDefault();
      e.stopImmediatePropagation();
      const text = e.clipboardData?.getData("text/plain") ?? "";
      void handlers.paste(text);
    },
    { capture: true },
  );
  fit(entry, terminal);
}

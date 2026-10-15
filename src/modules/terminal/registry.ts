import { applyTerminalTheme } from "./theme";
import { confirmPaste } from "./paste";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { desktop, errorMessage } from "../../services/desktop/client";
import {
  commands,
  isMac,
  modifierMatches,
} from "../../services/commands/registry";
import { useSettings } from "../settings/store";
import { loadTerminalFont, terminalFontFamily } from "../settings/fonts";
import { builtinSchemes, resolveScheme } from "../settings/themes";
import { useWorkspace, type TerminalTab } from "../workspace/store";
import type {
  SessionEvent,
  SessionSnapshot,
} from "../../services/desktop/types";
interface Stream {
  generation: number;
  sessionId?: string;
  seq: number;
  received: bigint;
  consumed: bigint;
  pending: number;
  acked: bigint;
  ack: Promise<void>;
  inputSeq: number;
  inputBytes: number;
  input: Promise<void>;
  ended: boolean;
  finalOffset?: bigint;
  ackTimer?: ReturnType<typeof setTimeout>;
  eventSeq: number;
}
interface Entry {
  term: Terminal;
  fit: FitAddon;
  search: SearchAddon;
  host: HTMLElement;
  observer: ResizeObserver;
  stream?: Stream;
  connecting: boolean;
  disposed: boolean;
  lastSize: string;
  fontRequest?: string;
}
const entries = new Map<string, Entry>();
function localLine(entry: Entry, message: string) {
  const safe = (text: string) => text.replace(/[\x00-\x1f\x7f-\x9f]/g, " ");
  let snapshot = "";
  if (entry.term.buffer.active.type === "alternate") {
    const buffer = entry.term.buffer.active;
    const lines: string[] = [];
    let bytes = 0;
    for (let row = 0; row < Math.min(1000, entry.term.rows); row++) {
      const line = safe(
        buffer.getLine(buffer.baseY + row)?.translateToString(true) ?? "",
      );
      bytes += new TextEncoder().encode(line).length;
      if (bytes > 1024 * 1024) {
        lines.push("[全屏画面快照超过 1 MiB，已截断]");
        break;
      }
      lines.push(line);
    }
    snapshot = "\r\n[断开前全屏画面快照]\r\n" + lines.join("\r\n") + "\r\n";
  }
  // v6 Utf8ToUtf32 对 ASCII 非续字节会丢弃残留；必须用 bytes 写 CAN，字符串使用另一解码器。
  // CAN 同时取消不完整控制序列；公开 buffer API 有界保留全屏末帧，再退出 alternate。
  const modes =
    "\x18\x1b[0m\x1b[?1049l\x1b[?2004l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l";
  entry.term.write(
    new TextEncoder().encode(
      modes +
        snapshot +
        `\r\n\x1b[90m[${new Date().toLocaleTimeString()}] ${safe(message)}\x1b[0m\r\n`,
    ),
  );
}
function update(tab: TerminalTab, snapshot: SessionSnapshot) {
  if (snapshot.stateRevision >= tab.revision) {
    tab.sessionId = snapshot.sessionId;
    tab.state = snapshot.state;
    tab.revision = snapshot.stateRevision;
  }
}
async function acknowledge(entry: Entry, stream: Stream) {
  if (!stream.sessionId || entry.disposed || stream.consumed <= stream.acked)
    return;
  const offset = stream.consumed;
  stream.acked = offset;
  stream.ack = stream.ack
    .then(() => desktop.ack(stream.sessionId!, offset.toString()))
    .catch((error) => {
      if (!stream.ended) {
        const tab = useWorkspace().tabs.find(
          (t) => t.terminalId === entry.host.dataset.terminalId,
        );
        if (tab) tab.error = errorMessage(error);
      }
    });
  await stream.ack;
}
function receive(entry: Entry, stream: Stream, raw: ArrayBuffer) {
  if (entry.stream !== stream || entry.disposed) return;
  try {
    if (!(raw instanceof ArrayBuffer) || raw.byteLength < 25)
      throw new Error("终端输出帧无效");
    const view = new DataView(raw),
      length = view.getUint32(16, true),
      seq = view.getUint32(4, true),
      offset = view.getBigUint64(8, true);
    if (
      view.getUint8(0) !== 1 ||
      view.getUint8(1) !== 0 ||
      view.getUint16(2, true) !== 0 ||
      view.getUint32(20, true) !== 0 ||
      length < 1 ||
      length > 32768 ||
      raw.byteLength !== 24 + length ||
      seq !== stream.seq + 1 ||
      offset !== stream.received + BigInt(length) ||
      stream.pending + length > 512 * 1024
    )
      throw new Error("PROTOCOL_MISMATCH：终端数据流不连续或超出预算");
    stream.seq = seq;
    stream.received = offset;
    stream.pending += length;
    entry.term.write(new Uint8Array(raw, 24, length), () => {
      stream.pending -= length;
      stream.consumed = offset;
      if (stream.consumed - stream.acked >= BigInt(65536)) {
        if (stream.ackTimer) clearTimeout(stream.ackTimer);
        stream.ackTimer = undefined;
        void acknowledge(entry, stream);
        return;
      }
      if (stream.ackTimer) return;
      stream.ackTimer = setTimeout(() => {
        stream.ackTimer = undefined;
        void acknowledge(entry, stream);
      }, 16);
    });
  } catch (error) {
    const tab = useWorkspace().tabs.find(
      (t) => t.terminalId === entry.host.dataset.terminalId,
    );
    if (tab) {
      tab.error = errorMessage(error);
      tab.state = "failed";
    }
    stream.ended = true;
    if (stream.sessionId)
      void desktop.close(stream.sessionId, "protocolMismatch").catch(() => {});
  }
}
function event(
  entry: Entry,
  tab: TerminalTab,
  stream: Stream,
  value: SessionEvent,
) {
  if (entry.stream !== stream || entry.disposed) return;
  stream.sessionId = value.sessionId;
  const seq = (value as SessionEvent & { eventSeq?: number }).eventSeq;
  if (seq !== undefined) {
    if (seq <= stream.eventSeq) return;
    stream.eventSeq = seq;
  }
  if (value.type === "stateChanged") {
    const wasConnected = tab.state === "connected";
    update(tab, value.snapshot);
    if (!wasConnected && tab.state === "connected") {
      // 认证期间布局可能已改变；PTY 建立后必须补发当前可见尺寸。
      entry.lastSize = "";
      fit(entry, tab);
    }
  } else if (
    value.type === "hostKeyChallenge" ||
    value.type === "authChallenge"
  ) {
    const workspace = useWorkspace();
    workspace.dismiss(value.sessionId);
    workspace.challenges.push(value);
  } else if (value.type === "ended") {
    stream.ended = true;
    stream.finalOffset = BigInt(value.finalOffset);
    tab.state = value.finalState;
    tab.error =
      value.error?.message ??
      (!value.outputComplete ? "连接结束，末尾输出可能不完整" : "");
    useWorkspace().dismiss(value.sessionId);
    // 等待最后一帧解析，控制事件与数据 Channel 没有全局先后顺序。
    void drain(entry, stream)
      .then(() => {
        if (entry.stream === stream)
          localLine(
            entry,
            `${value.reason}${value.exitCode !== undefined && value.exitCode !== null ? `（退出码 ${value.exitCode}）` : ""}`,
          );
      })
      .catch((error) => {
        tab.error = errorMessage(error);
      });
  } else if (value.type === "notice")
    tab.error = value.details ?? value.messageKey;
}
function drain(entry: Entry, stream: Stream): Promise<void> {
  return new Promise((resolve, reject) => {
    const deadline = Date.now() + 3000;
    const check = () => {
      if (entry.disposed) return resolve();
      if (
        stream.pending === 0 &&
        (stream.finalOffset === undefined ||
          stream.consumed >= stream.finalOffset)
      ) {
        const timer = setTimeout(
          () => reject(new Error("终端解析栅栏超时，请稍后重试；历史已保留。")),
          Math.max(1, deadline - Date.now()),
        );
        entry.term.write(new Uint8Array([0x18]), () => {
          clearTimeout(timer);
          resolve();
        });
        return;
      }
      if (Date.now() >= deadline) {
        reject(new Error("旧终端输出尚未排空，请稍后重试；历史已保留。"));
        return;
      }
      setTimeout(check, 20);
    };
    check();
  });
}
function input(entry: Entry, tab: TerminalTab, bytes: Uint8Array) {
  const stream = entry.stream;
  if (!stream || stream.ended || tab.state !== "connected" || !stream.sessionId)
    return;
  if (stream.inputBytes + bytes.length > 65536) {
    tab.error = "输入队列已满，请等待后重试粘贴。";
    return;
  }
  stream.inputBytes += bytes.length;
  for (let at = 0; at < bytes.length; at += 16384) {
    const chunk = bytes.slice(at, at + 16384);
    stream.input = stream.input.then(async () => {
      try {
        if (
          entry.stream !== stream ||
          stream.ended ||
          tab.state !== "connected"
        )
          return;
        if (stream.inputSeq >= 0xffffffff)
          throw new Error("输入序号已耗尽，请重连。");
        await desktop.write(
          stream.sessionId!,
          ++stream.inputSeq,
          Array.from(chunk),
        );
      } catch (error) {
        stream.ended = true;
        tab.error = `${errorMessage(error)}；未确认输入不会自动重放。`;
        if (stream.sessionId)
          void desktop.close(stream.sessionId, "inputFailed").catch(() => {});
      } finally {
        stream.inputBytes -= chunk.length;
      }
    });
  }
}
function settings(entry: Entry, tab: TerminalTab) {
  const s = useSettings().value;
  applyTerminalTheme(entry.term, entry.host, schemeForTab(tab));
  const family = terminalFontFamily(s.fontFamily);
  const request = `${s.fontSize}px ${family}`;
  if (entry.fontRequest !== request) {
    entry.fontRequest = request;
    void loadTerminalFont(s.fontFamily, s.fontSize)
      .then(() => {
        // 迟到的加载不能覆盖较新的选择，也不能访问已关闭的终端。
        if (entry.disposed || entry.fontRequest !== request) return;
        entry.term.options.fontFamily = family;
        fit(entry, tab);
      })
      .catch(() => {
        if (entry.disposed || entry.fontRequest !== request) return;
        entry.fontRequest = undefined;
        tab.error = "内置终端字体加载失败，请重新应用设置或重启应用。";
      });
  }
  entry.term.options.fontSize = s.fontSize;
  entry.term.options.scrollback = s.scrollback;
}
export function schemeForTab(tab: TerminalTab) {
  const s = useSettings().value;
  return resolveScheme(
    tab.schemeId ??
      (tab.target.source === "saved"
        ? s.hostSchemes[tab.target.profileId]
        : undefined) ??
      s.defaultSchemeId,
    s.customSchemes,
  );
}
function fit(entry: Entry, tab: TerminalTab) {
  if (entry.host.clientWidth < 10 || entry.host.clientHeight < 10) return;
  entry.fit.fit();
  const size = {
    cols: Math.min(1000, entry.term.cols),
    rows: Math.min(1000, entry.term.rows),
  };
  tab.size = size;
  const key = `${size.cols}:${size.rows}`;
  if (entry.lastSize !== key && tab.sessionId && tab.state === "connected") {
    entry.lastSize = key;
    void desktop.resize(tab.sessionId, size).catch((error) => {
      entry.lastSize = "";
      tab.error = errorMessage(error);
    });
  }
}
export const terminalRegistry = {
  mount(tab: TerminalTab, host: HTMLElement) {
    if (entries.has(tab.terminalId)) return;
    const term = new Terminal({
      // 粗体仅改变字重，避免普通 ANSI 色被自动替换为高亮色。
      drawBoldTextInBrightColors: false,
      convertEol: false,
      cursorBlink: true,
      allowProposedApi: false,
      scrollback: 5000,
      screenReaderMode: true,
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
            tab.error = errorMessage(error);
          });
        }, 0);
      }
    };
    term.loadAddon(new WebLinksAddon(activate));
    term.options.linkHandler = { activate };
    host.dataset.terminalId = tab.terminalId;
    const entry: Entry = {
      term,
      fit: fitAddon,
      search,
      host,
      observer: new ResizeObserver(() => fit(entry, tab)),
      connecting: false,
      disposed: false,
      lastSize: "",
    };
    entries.set(tab.terminalId, entry);
    settings(entry, tab);
    term.open(host);
    entry.observer.observe(host);
    term.attachCustomKeyEventHandler((e) => {
      if (e.type !== "keydown") return true;
      // 单独按修饰键不应结束输入法组合、清除选区或改变滚屏位置。
      if (["Meta", "Control", "Alt", "Shift"].includes(e.key)) return false;
      if (commands.route(e, useSettings().value.keymap.bindings, true))
        return false;
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
    term.onData((data) => input(entry, tab, new TextEncoder().encode(data)));
    term.onBinary((data) =>
      input(
        entry,
        tab,
        Uint8Array.from(data, (c) => c.charCodeAt(0)),
      ),
    );
    host.addEventListener(
      "paste",
      (e) => {
        e.preventDefault();
        e.stopImmediatePropagation();
        const text = e.clipboardData?.getData("text/plain") ?? "";
        void terminalRegistry.pasteText(tab.terminalId, text);
      },
      { capture: true },
    );
    fit(entry, tab);
    void this.connect(tab);
  },
  async connect(tab: TerminalTab) {
    const entry = entries.get(tab.terminalId);
    if (!entry || entry.connecting) return;
    entry.connecting = true;
    try {
      const old = entry.stream;
      if (old) {
        if (old.sessionId && !old.ended) {
          await desktop.close(old.sessionId, "reconnect");
          const deadline = Date.now() + 3000;
          while (!old.ended && Date.now() < deadline)
            await new Promise((r) => setTimeout(r, 20));
          if (!old.ended) throw new Error("旧会话仍在关闭，请稍后重试。");
        }
        await drain(entry, old);
        if (old.ackTimer) clearTimeout(old.ackTimer);
        await acknowledge(entry, old);
        useWorkspace().dismiss(old.sessionId ?? "");
      }
      localLine(entry, old ? "重新连接，保留终端历史" : "开始 SSH 连接");
      const stream: Stream = {
        generation: (old?.generation ?? 0) + 1,
        seq: 0,
        received: BigInt(0),
        consumed: BigInt(0),
        acked: BigInt(0),
        ack: Promise.resolve(),
        pending: 0,
        inputSeq: 0,
        inputBytes: 0,
        input: Promise.resolve(),
        ended: false,
        eventSeq: 0,
      };
      entry.stream = stream;
      tab.state = "connecting";
      tab.error = "";
      tab.revision = 0;
      tab.sessionId = undefined;
      const snapshot = await desktop.openSession(
        tab.terminalId,
        tab.target,
        tab.size,
        (data) => receive(entry, stream, data),
        (value) => event(entry, tab, stream, value),
      );
      if (entry.disposed) {
        await desktop.close(snapshot.sessionId, "tabClosed");
        return;
      }
      stream.sessionId = snapshot.sessionId;
      update(tab, snapshot);
      await acknowledge(entry, stream);
    } catch (error) {
      tab.error = errorMessage(error);
      tab.state = "failed";
      localLine(entry, tab.error);
    } finally {
      entry.connecting = false;
    }
  },
  async disconnect(id: string) {
    const e = entries.get(id),
      tab = useWorkspace().tabs.find((t) => t.terminalId === id);
    if (e?.stream?.sessionId && !e.stream.ended) {
      if (tab) tab.state = "closing";
      const snapshot = await desktop.close(e.stream.sessionId);
      if (tab) update(tab, snapshot);
    }
  },
  async dispose(id: string) {
    const e = entries.get(id);
    if (!e) return;
    await this.disconnect(id);
    e.disposed = true;
    e.observer.disconnect();
    if (e.stream?.ackTimer) clearTimeout(e.stream.ackTimer);
    e.term.dispose();
    entries.delete(id);
  },
  apply() {
    for (const tab of useWorkspace().tabs) {
      const e = entries.get(tab.terminalId);
      if (e) {
        const available = [
          ...useSettings().value.customSchemes.map((s) => s.id),
          ...builtinSchemes.map((s) => s.id),
        ];
        if (tab.schemeId && !available.includes(tab.schemeId))
          tab.schemeId = undefined;
        settings(e, tab);
        fit(e, tab);
      }
    }
  },
  focus(id: string) {
    const e = entries.get(id),
      tab = useWorkspace().tabs.find((t) => t.terminalId === id);
    if (e && tab) {
      requestAnimationFrame(() => {
        // 快速切换到首页或弹出认证时，迟到的焦点任务不能抢回隐藏终端。
        if (
          e.disposed ||
          useWorkspace().activeId !== id ||
          document.querySelector("[data-modal-open], [data-ui-popup-open]")
        )
          return;
        fit(e, tab);
        e.term.focus();
      });
    }
  },
  async copy(id: string) {
    const text = entries.get(id)?.term.getSelection();
    if (text) await navigator.clipboard.writeText(text);
  },
  async paste(id: string) {
    await this.pasteText(id, await navigator.clipboard.readText());
  },
  async pasteText(id: string, text: string) {
    const e = entries.get(id),
      tab = useWorkspace().tabs.find((t) => t.terminalId === id);
    if (!e || !tab || tab.state !== "connected") return;
    if (new TextEncoder().encode(text).length > 65536) {
      tab.error = "单次粘贴不能超过 64 KiB。";
      return;
    }
    const stream = e.stream;
    if (
      useSettings().value.multilinePaste &&
      /[\r\n]/.test(text) &&
      !(await confirmPaste(text))
    )
      return;
    if (
      e.disposed ||
      e.stream !== stream ||
      stream?.ended ||
      tab.state !== "connected"
    )
      return;
    e.term.paste(text);
  },
  find(id: string, query: string, previous = false) {
    const search = entries.get(id)?.search;
    if (query)
      return previous ? search?.findPrevious(query) : search?.findNext(query);
    search?.clearDecorations();
  },
  bottom(id: string) {
    entries.get(id)?.term.scrollToBottom();
  },
};

import { t } from "@/i18n";
import { errorMessage } from "@/services/desktop/client";
import { nextTick } from "vue";
import { sendFinalWheelInput } from "./inputOrigin";
import { createInputRouter, type InputKind } from "./inputRouter";
import { confirmPaste } from "./paste";
import { terminalIds } from "../workbench/layout";
import { useSettings } from "../settings/store";
import { builtinSchemes, resolveScheme } from "@/theme/schemes";
import { useWorkbenchStore, type TerminalItem } from "../workbench/store";
import type { Entry } from "./runtime/entry";
import { input } from "./runtime/stream";
import { connect, disconnect, archiveAndDispose } from "./runtime/session";
import {
  createTerminalEntry,
  mountTerminalView,
  remoteWheel,
  settings,
  fit,
} from "./runtime/view";

const entries = new Map<string, Entry>();
let archiving = 0;
const archiveWaiters: (() => void)[] = [];
async function archive(entry: Entry) {
  // 所有关闭入口共享预算，窗口退出与手动关闭重叠也最多保留两份滚屏快照。
  if (archiving >= 2)
    await new Promise<void>((resolve) => archiveWaiters.push(resolve));
  else archiving++;
  try {
    await archiveAndDispose(entry, entry.terminal);
  } finally {
    const next = archiveWaiters.shift();
    if (next) next();
    else archiving--;
  }
}
function connectedTargets(sourceId: string, kind: InputKind = "keyboard") {
  const workbench = useWorkbenchStore();
  const source = workbench.terminals.find(
    (terminal) => terminal.terminalId === sourceId,
  );
  const origin = entries.get(sourceId);
  if (
    source?.state !== "connected" ||
    !origin?.stream?.control.sessionId ||
    origin.stream.inputStopped ||
    origin.stream.control.ended ||
    origin.disposed
  )
    return [];
  const ids = document.querySelector("[data-modal-open], [data-ui-popup-open]")
    ? [sourceId]
    : workbench.inputTargets(sourceId, kind);
  return ids.filter((id) => {
    const terminal = workbench.terminals.find((item) => item.terminalId === id);
    const entry = entries.get(id);
    return (
      terminal?.state === "connected" &&
      entry &&
      !entry.disposed &&
      entry.stream?.control.sessionId &&
      !entry.stream.inputStopped &&
      !entry.stream.control.ended &&
      !workbench.closingTerminalIds.includes(id)
    );
  });
}
function punctuationConversionEnabled(id: string) {
  return (
    entries.get(id)?.terminal.convertChinesePunctuation ??
    useSettings().value.convertChinesePunctuation
  );
}
const inputRouter = createInputRouter({
  targets: connectedTargets,
  convertChinesePunctuation: punctuationConversionEnabled,
  send(id, bytes) {
    const entry = entries.get(id);
    if (entry && !entry.disposed) input(entry, entry.terminal, bytes);
  },
  applicationCursor: (id) =>
    entries.get(id)?.term.modes.applicationCursorKeysMode ?? false,
  wheel(id, final, sourceId, event) {
    const target = entries.get(id)?.term;
    const source = entries.get(sourceId)?.term;
    if (!target?.element || !source?.element || !remoteWheel(target)) return;
    sendFinalWheelInput(target, final, source, event);
  },
});
export const terminalRegistry = {
  mount(terminal: TerminalItem, host: HTMLElement) {
    if (entries.has(terminal.terminalId)) return;
    const entry = createTerminalEntry(terminal, host);
    entries.set(terminal.terminalId, entry);
    mountTerminalView(entry, terminal, {
      data: (packet) => inputRouter.data(terminal.terminalId, packet),
      punctuationEnabled: () =>
        punctuationConversionEnabled(terminal.terminalId),
      paste: (text) => this.pasteText(terminal.terminalId, text),
      reconnect: () => this.connect(terminal),
      pasteClipboard: () => this.paste(terminal.terminalId),
    });
    void this.connect(terminal);
  },
  async connect(terminal: TerminalItem) {
    const entry = entries.get(terminal.terminalId);
    if (entry) await connect(entry, terminal);
  },
  async disconnect(id: string) {
    const entry = entries.get(id);
    if (entry) await disconnect(entry, entry.terminal);
  },
  async closeAndRemove(id: string) {
    const workbench = useWorkbenchStore();
    const entry = entries.get(id);
    if (!entry) {
      workbench.remove(id);
      return;
    }
    if (!entry.closing) {
      workbench.closingTerminalIds.push(id);
      // 先发布关闭事实，再执行异步操作；普通关闭与窗口退出共用这一入口。
      entry.closing = Promise.resolve()
        .then(() => archive(entry))
        .then(() => {
          entries.delete(id);
          workbench.remove(id);
        })
        .catch((cause) => {
          entry.terminal.error = errorMessage(cause);
          throw cause;
        })
        .finally(() => {
          entry.closing = undefined;
          workbench.closingTerminalIds = workbench.closingTerminalIds.filter(
            (value) => value !== id,
          );
        });
    }
    await entry.closing;
  },
  async closeMany(ids: string[]) {
    // 单项失败仍继续提交其余终端，调用方决定是否放行窗口退出。
    return Promise.allSettled(ids.map((id) => this.closeAndRemove(id)));
  },
  apply() {
    const workbench = useWorkbenchStore();
    const saved = useSettings().value;
    const available = new Set([
      ...saved.customSchemes.map((s) => s.id),
      ...builtinSchemes.map((s) => s.id),
    ]);
    for (const terminal of workbench.terminals) {
      if (terminal.schemeId && !available.has(terminal.schemeId)) {
        const group = workbench.groupForTerminal(terminal.terminalId);
        // 删除自定义配色后工作区仍统一回退；单终端继续沿用原来的继承规则。
        terminal.schemeId =
          group && (group.root.kind === "split" || group.savedProfileId)
            ? resolveScheme(saved.defaultSchemeId, saved.customSchemes).id
            : undefined;
      }
      const e = entries.get(terminal.terminalId);
      if (e) {
        settings(e, terminal);
        fit(e, terminal);
      }
    }
  },
  focus(id: string) {
    const e = entries.get(id),
      terminal = useWorkbenchStore().terminals.find((t) => t.terminalId === id);
    if (e && terminal) {
      requestAnimationFrame(() => {
        // 快速切换到保险库或弹出认证时，迟到的焦点任务不能抢回隐藏终端。
        if (
          e.disposed ||
          useWorkbenchStore().activeTerminal?.terminalId !== id ||
          document.querySelector("[data-modal-open], [data-ui-popup-open]")
        )
          return;
        fit(e, terminal);
        e.term.focus();
      });
    }
  },
  async copy(id: string) {
    const text = entries.get(id)?.term.getSelection();
    if (text) await navigator.clipboard.writeText(text);
  },
  async paste(id: string) {
    const workbench = useWorkbenchStore();
    const group = workbench.activeGroup;
    const revision = group?.broadcastRevision;
    const stream = entries.get(id)?.stream;
    const text = await navigator.clipboard.readText();
    if (
      workbench.activeGroup !== group ||
      group?.broadcastRevision !== revision ||
      entries.get(id)?.stream !== stream
    )
      return;
    await this.pasteText(id, text);
  },
  async pasteText(
    id: string,
    text: string,
    options: {
      broadcast?: boolean;
      execute?: boolean;
      expectedSessionId?: string;
    } = {},
  ) {
    const workbench = useWorkbenchStore();
    const e = entries.get(id);
    const terminal = workbench.terminals.find((item) => item.terminalId === id);
    if (
      !e ||
      !terminal ||
      terminal.state !== "connected" ||
      workbench.activeTerminal?.terminalId !== id ||
      (options.expectedSessionId !== undefined &&
        e.stream?.control.sessionId !== options.expectedSessionId)
    )
      return;
    if (new TextEncoder().encode(text).length > 65536) {
      terminal.error = t("terminal.paste.tooLarge");
      return;
    }
    const group = workbench.activeGroup!;
    const revision = group.broadcastRevision;
    const sourceStream = e.stream;
    const targets = (
      options.broadcast === false ? [id] : connectedTargets(id)
    ).map((targetId) => ({
      id: targetId,
      entry: entries.get(targetId)!,
      stream: entries.get(targetId)!.stream,
    }));
    if (
      useSettings().value.multilinePaste &&
      /[\r\n]/.test(text) &&
      !(await confirmPaste(text, targets.length))
    )
      return;
    await nextTick();
    // 确认期间切组/切焦点、修改广播状态或成员均取消，不把旧输入交给新会话。
    if (
      e.disposed ||
      e.stream !== sourceStream ||
      sourceStream?.inputStopped ||
      sourceStream?.control.ended ||
      terminal.state !== "connected" ||
      workbench.activeGroup !== group ||
      workbench.activeTerminal?.terminalId !== id ||
      group.broadcastRevision !== revision ||
      document.querySelector("[data-modal-open], [data-ui-popup-open]")
    )
      return;
    const pasteBytes = new TextEncoder().encode(
      text.replace(/\r?\n/g, "\r"),
    ).length;
    for (const target of targets) {
      const current = workbench.terminals.find(
        (item) => item.terminalId === target.id,
      );
      if (
        target.entry.disposed ||
        target.entry.stream !== target.stream ||
        target.stream?.inputStopped ||
        target.stream?.control.ended ||
        current?.state !== "connected" ||
        workbench.closingTerminalIds.includes(target.id)
      )
        continue;
      // 先为正文、括号标记和执行回车一起检查队列，避免正文被拒后只发送回车。
      const requiredBytes =
        pasteBytes +
        (target.entry.term.modes.bracketedPasteMode &&
        !target.entry.term.options.ignoreBracketedPasteMode
          ? 12
          : 0) +
        (options.execute ? 1 : 0);
      if (!target.stream || target.stream.inputBytes + requiredBytes > 65536) {
        if (current) current.error = t("terminal.paste.queueFull");
        continue;
      }
      // 每个目标独立应用括号粘贴模式；本次已展开目标，不能再次触发广播。
      inputRouter.local(target.id, () => {
        target.entry.term.paste(text);
        // 回车在括号粘贴结束后单独发送，与正文共用本次目标及会话代次校验。
        if (options.execute)
          input(target.entry, target.entry.terminal, Uint8Array.of(13));
      });
    }
  },
  toggleBroadcast() {
    const workbench = useWorkbenchStore();
    const group = workbench.activeGroup;
    if (!group) return;
    if (
      !group.broadcast &&
      terminalIds(group.root).some(
        (id) =>
          !workbench.terminals.find((terminal) => terminal.terminalId === id)
            ?.sshCommand && !entries.get(id)?.inputOrigin?.supported,
      )
    ) {
      if (workbench.activeTerminal)
        workbench.activeTerminal.error = t("workbench.broadcast.unsupported");
      return;
    }
    workbench.toggleBroadcast();
    if (workbench.activeTerminal)
      this.focus(workbench.activeTerminal.terminalId);
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

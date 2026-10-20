import { t } from "../../../i18n";
import { errorMessage } from "../../../services/desktop/client";
import {
  registerSessionPresence,
  unregisterSessionPresence,
} from "../../../services/desktop/sessionPresence";
import type {
  SessionEvent,
  SessionSnapshot,
} from "../../../services/desktop/types";
import { useHosts } from "../../hosts/store";
import { useWorkbenchStore, type TerminalItem } from "../../workbench/store";
import { resetTerminalActivity } from "../activity";
import { writeLocalStatus } from "../localStatus";
import type { Entry, Stream } from "./entry";
import { drain } from "./stream";
import { fit } from "./view";

export function update(terminal: TerminalItem, snapshot: SessionSnapshot) {
  if (snapshot.stateRevision >= terminal.revision) {
    if (terminal.sessionId && terminal.sessionId !== snapshot.sessionId)
      unregisterSessionPresence("terminal", terminal.sessionId);
    terminal.kind = snapshot.kind;
    terminal.sshCommand = snapshot.sshCommand;
    terminal.sessionId = snapshot.sessionId;
    terminal.state = snapshot.state;
    terminal.revision = snapshot.stateRevision;
    if (["closed", "failed", "disconnected"].includes(snapshot.state))
      unregisterSessionPresence("terminal", snapshot.sessionId);
    else registerSessionPresence("terminal", snapshot.sessionId);
  }
}
export function event(
  entry: Entry,
  terminal: TerminalItem,
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
  if (value.type === "hostSystemChanged") {
    // 后端先提交持久化状态；通知只触发重读，不接受远端自定义图标内容。
    void useHosts()
      .load()
      .catch((error) => {
        if (entry.stream === stream && !entry.disposed)
          terminal.error = errorMessage(error);
      });
  } else if (value.type === "stateChanged") {
    entry.canResize = value.snapshot.capabilities.resize;
    const wasConnected = terminal.state === "connected";
    update(terminal, value.snapshot);
    if (!wasConnected && terminal.state === "connected") {
      // 认证期间布局可能已改变；PTY 建立后必须补发当前可见尺寸。
      entry.lastSize = "";
      fit(entry, terminal);
    }
  } else if (
    value.type === "hostKeyChallenge" ||
    value.type === "authChallenge"
  ) {
    const workbench = useWorkbenchStore();
    workbench.dismiss(value.sessionId);
    workbench.challenges.push(value);
  } else if (value.type === "ended") {
    const workbench = useWorkbenchStore();
    unregisterSessionPresence("terminal", value.sessionId);
    stream.ended = true;
    stream.endReceived = true;
    stream.finalOffset = BigInt(value.finalOffset);
    terminal.state = value.finalState;
    // 主动关闭时，取消产生的未完整输出是收尾过程，不应在标签移除前闪成错误。
    // dispose/日志保存真正失败仍会由关闭协调器写入 error，保留给用户处理。
    terminal.error = workbench.closingTerminalIds.includes(terminal.terminalId)
      ? ""
      : value.error
        ? errorMessage(value.error)
        : !value.outputComplete
          ? t("terminal.state.incompleteOutput")
          : "";
    workbench.dismiss(value.sessionId);
    // 等待最后一帧解析，控制事件与数据 Channel 没有全局先后顺序。
    stream.finalization = drain(entry, stream).then(() => {
      if (entry.stream === stream && !entry.disposed && !entry.closing) {
        resetTerminalActivity(terminal);
        writeLocalStatus(
          entry.term,
          [
            terminal.sshCommand && value.reason === "remoteExit"
              ? t("terminal.state.commandFinished")
              : t("terminal.state.ended"),
            value.exitCode !== undefined && value.exitCode !== null
              ? t("terminal.exit.code", { exitCode: value.exitCode })
              : "",
            value.exitSignal
              ? t("terminal.exit.signal", { exitSignal: value.exitSignal })
              : "",
          ].join(""),
        );
        return new Promise<void>((resolve) => entry.term.write("", resolve));
      }
    });
    void stream.finalization.catch((error) => {
      if (
        entry.stream === stream &&
        !entry.disposed &&
        !useWorkbenchStore().closingTerminalIds.includes(terminal.terminalId)
      )
        terminal.error = errorMessage(error);
    });
  } else if (value.type === "notice")
    terminal.error = value.details ?? value.messageKey;
}

import { t } from "../../../i18n";
import { desktop, errorMessage } from "../../../services/desktop/client";
import { useHosts } from "../../hosts/store";
import { useWorkbenchStore, type TerminalItem } from "../../workbench/store";
import { captureTerminalLog } from "../../session-logs/snapshot";
import { resetTerminalActivity } from "../activity";
import { writeLocalStatus } from "../localStatus";
import type { Entry } from "./entry";
import { update, event } from "./events";
import { createStream, receive, acknowledge, drain } from "./stream";

export async function connect(entry: Entry, terminal: TerminalItem) {
  if (entry.connecting || entry.closing || entry.disposed) return;
  entry.connecting = true;
  try {
    const old = entry.stream;
    if (old) {
      if (old.sessionId && !old.ended) {
        await desktop.close(old.sessionId, "reconnect");
        const deadline = Date.now() + 3000;
        while (!old.ended && Date.now() < deadline)
          await new Promise((r) => setTimeout(r, 20));
        if (!old.ended) throw new Error(t("terminal.errors.sessionClosing"));
      }
      await drain(entry, old);
      if (old.ackTimer) clearTimeout(old.ackTimer);
      await acknowledge(entry, terminal, old);
      useWorkbenchStore().dismiss(old.sessionId ?? "");
    }
    if (terminal.target.source === "saved") {
      const profileId = terminal.target.profileId;
      const profile = useHosts().hosts.find((host) => host.id === profileId);
      if (profile?.kind === "workspace")
        throw new Error(t("workspaces.errors.singleConnection"));
      if (profile) {
        terminal.kind = profile.kind;
        terminal.serialConfig = profile.serialConfig
          ? { ...profile.serialConfig }
          : undefined;
      }
    }
    resetTerminalActivity(terminal);
    writeLocalStatus(
      entry.term,
      terminal.kind === "local"
        ? old
          ? t("terminal.actions.reopenLocal")
          : t("terminal.state.startingLocal")
        : terminal.kind === "serial"
          ? old
            ? t("serial.actions.reopen")
            : t("serial.actions.open")
          : old
            ? t("terminal.actions.reconnectWithHistory")
            : terminal.kind === "telnet"
              ? t("terminal.state.startingTelnet")
              : t("terminal.state.startingSsh"),
    );
    const stream = createStream();
    entry.stream = stream;
    terminal.state = "connecting";
    terminal.error = "";
    terminal.revision = 0;
    terminal.sessionId = undefined;
    const snapshot = await desktop.openSession(
      terminal.terminalId,
      terminal.target,
      terminal.size,
      (data) => receive(entry, terminal, stream, data),
      (value) => event(entry, terminal, stream, value),
    );
    if (entry.disposed) {
      await desktop.close(snapshot.sessionId, "tabClosed");
      return;
    }
    stream.sessionId = snapshot.sessionId;
    entry.canResize = snapshot.capabilities.resize;
    update(terminal, snapshot);
    await acknowledge(entry, terminal, stream);
  } catch (error) {
    terminal.error = errorMessage(error);
    terminal.state = "failed";
    writeLocalStatus(entry.term, terminal.error);
  } finally {
    entry.connecting = false;
  }
}
export async function disconnect(entry: Entry, terminal?: TerminalItem) {
  const stream = entry.stream;
  if (stream?.sessionId && !stream.endReceived) {
    if (terminal) terminal.state = "closing";
    const snapshot = await desktop.close(stream.sessionId);
    if (terminal && entry.stream === stream && !entry.disposed)
      update(terminal, snapshot);
  }
}
export async function archiveAndDispose(entry: Entry, terminal: TerminalItem) {
  // 连接命令返回后才能可靠取消；超时保留视图和缓冲，允许再次关闭。
  const deadline = Date.now() + 10000;
  while (entry.connecting && Date.now() < deadline)
    await new Promise((resolve) => setTimeout(resolve, 20));
  if (entry.connecting) throw new Error(t("terminal.errors.sessionClosing"));
  await disconnect(entry, terminal);
  const stream = entry.stream;
  if (stream?.sessionId) {
    const endDeadline = Date.now() + 5000;
    while (!stream.endReceived && Date.now() < endDeadline)
      await new Promise((resolve) => setTimeout(resolve, 20));
    if (!stream.endReceived)
      throw new Error(t("terminal.errors.sessionClosing"));
    // 较早的排空可能因迟到帧超时；本次重新排空成功后仍允许安全重试关闭。
    await stream.finalization?.catch(() => {});
    await drain(entry, stream);
    await acknowledge(entry, terminal, stream);
  } else {
    await new Promise<void>((resolve) => entry.term.write("", resolve));
  }
  // 数据库确认提交之前保留 xterm；同一 terminalId 重试不会重复插入。
  await desktop.saveSessionLog({
    ...entry.log,
    content: captureTerminalLog(entry.term),
  });
  entry.disposed = true;
  entry.observer.disconnect();
  if (stream?.ackTimer) clearTimeout(stream.ackTimer);
  entry.inputOrigin?.dispose();
  entry.term.dispose();
}

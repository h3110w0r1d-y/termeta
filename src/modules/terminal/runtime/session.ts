import { t } from "@/i18n";
import { desktop, errorMessage } from "@/services/desktop/client";
import { useHosts } from "@/modules/hosts/store";
import type { TerminalItem } from "@/modules/workbench/store";
import { useSessions } from "@/modules/sessions/store";
import { captureTerminalLog } from "@/modules/session-logs/snapshot";
import { resetTerminalActivity } from "../activity";
import { writeLocalStatus } from "../localStatus";
import type { Entry } from "./entry";
import { update, event } from "./events";
import { createStream, receive, acknowledge, drain } from "./stream";

/** 连接和关闭都由 Registry 持有的 Entry 合并，布局变化不创建第二套生命周期。 */
export function connect(entry: Entry, terminal: TerminalItem) {
  if (entry.connection) return entry.connection;
  if (entry.closing || entry.disposed) return Promise.resolve();
  const connection = Promise.resolve()
    .then(() => open(entry, terminal))
    .finally(() => {
      if (entry.connection === connection) entry.connection = undefined;
    });
  entry.connection = connection;
  return connection;
}

async function waitFor<T>(promise: Promise<T>, timeout: number) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error(t("terminal.errors.sessionClosing"))),
          timeout,
        );
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

async function open(entry: Entry, terminal: TerminalItem) {
  if (entry.closing || entry.disposed) return;
  try {
    const old = entry.stream;
    if (old) {
      if (old.control.sessionId && !old.control.ended) {
        await desktop.close(old.control.sessionId, "reconnect");
        await waitFor(old.whenEnded, 3000);
      }
      await old.finalization?.catch(() => {});
      await drain(entry, old);
      if (old.ackTimer) clearTimeout(old.ackTimer);
      await acknowledge(entry, terminal, old);
      if (old.control.sessionId)
        useSessions().release("terminal", old.control.sessionId);
    }
    // 旧流收尾期间用户可能已关闭视图，此时不再创建下一次连接。
    if (entry.closing || entry.disposed) return;
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
      stream.control.requestId,
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
    if (
      snapshot.requestId !== stream.control.requestId ||
      (stream.control.sessionId &&
        stream.control.sessionId !== snapshot.sessionId)
    )
      throw new Error(t("terminal.errors.outputFailure"));
    if (update(terminal, stream, snapshot))
      entry.canResize = snapshot.capabilities.resize;
    await acknowledge(entry, terminal, stream);
  } catch (error) {
    // 控制事件先报告终态时，命令响应丢失不能再把已结束状态改为连接失败。
    if (entry.stream?.control.ended) return;
    terminal.error = errorMessage(error);
    terminal.state = "failed";
    writeLocalStatus(entry.term, terminal.error);
  }
}
export async function disconnect(entry: Entry, terminal?: TerminalItem) {
  if (entry.connection) await waitFor(entry.connection, 10000);
  const stream = entry.stream;
  if (stream?.control.sessionId && !stream.control.ended) {
    if (terminal) terminal.state = "closing";
    const snapshot = await desktop.close(stream.control.sessionId);
    if (terminal && entry.stream === stream && !entry.disposed)
      update(terminal, stream, snapshot);
  }
}
export async function archiveAndDispose(entry: Entry, terminal: TerminalItem) {
  // 等待在途打开的完成信号；超时仍保留视图，允许命令返回后再次关闭。
  if (entry.connection) await waitFor(entry.connection, 10000);
  await disconnect(entry, terminal);
  const stream = entry.stream;
  if (stream?.control.sessionId) {
    await waitFor(stream.whenEnded, 5000);
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
  if (stream?.control.sessionId)
    useSessions().release("terminal", stream.control.sessionId);
  entry.disposed = true;
  entry.observer.disconnect();
  if (stream?.ackTimer) clearTimeout(stream.ackTimer);
  entry.inputOrigin?.dispose();
  entry.disposeMouseActions?.();
  entry.term.dispose();
}

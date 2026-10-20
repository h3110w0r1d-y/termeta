import { onBeforeUnmount, onMounted, ref, type Ref } from "vue";
import { t } from "../i18n";
import { confirmAction } from "../components/ui/confirmation";
import { usePortForwards } from "../modules/forwarding/store";
import { useSettings } from "../modules/settings/store";
import { needsCloseConfirmation } from "../modules/terminal/activity";
import { terminalRegistry } from "../modules/terminal/registry";
import { useWorkbenchStore } from "../modules/workbench/store";
import { desktop, errorMessage } from "../services/desktop/client";
import { sessionPresenceSnapshot } from "../services/desktop/sessionPresence";

/** 主窗口负责退出握手与会话心跳，设置窗口不参与会话所有权。 */
export function useMainWindowLifecycle(options: {
  vaultReady: Ref<boolean>;
  beforeSave: () => void;
  reportError: (message: string) => void;
}) {
  const workbench = useWorkbenchStore();
  const settings = useSettings();
  const portForwards = usePortForwards();
  const savingLogs = ref(false);
  let disposed = false;
  let closingWindow = false;
  let preparingClose = false;
  let exitingApp = false;
  let presenceSync: ReturnType<typeof setInterval> | undefined;
  const listeners: (() => void)[] = [];

  async function closeApplication() {
    if (preparingClose || closingWindow) return;
    preparingClose = true;
    try {
      if (
        (workbench.fileTransferring ||
          workbench.terminals.some((terminal) => terminal.sftpTransferring)) &&
        !(await confirmAction(t("window.close.confirmTransfers")))
      )
        return;
      const running = workbench.terminals.filter(needsCloseConfirmation);
      if (
        (running.length > 0 || portForwards.hasActive) &&
        settings.value.confirmClose &&
        !(await confirmAction(
          [
            running.length
              ? t("window.close.runningTerminals", { count: running.length })
              : "",
            portForwards.hasActive ? t("window.close.activeForwarding") : "",
            t("window.close.confirmSessions"),
          ]
            .filter(Boolean)
            .join(" "),
        ))
      )
        return;
      options.beforeSave();
      savingLogs.value = true;
      // 任一日志提交失败都保留对应视图，并阻止窗口或应用退出。
      const results = await Promise.allSettled(
        workbench.terminals.map(async (terminal) => {
          await terminalRegistry.dispose(terminal.terminalId);
          workbench.dismiss(terminal.sessionId ?? "");
          workbench.remove(terminal.terminalId);
        }),
      );
      const failure = results.find((result) => result.status === "rejected");
      if (failure?.status === "rejected") throw failure.reason;
      if (exitingApp) {
        await desktop.finishAppExit();
      } else {
        closingWindow = true;
        await desktop.closeWindow();
      }
    } catch (cause) {
      closingWindow = false;
      options.reportError(errorMessage(cause));
    } finally {
      savingLogs.value = false;
      preparingClose = false;
      exitingApp = false;
    }
  }

  function retainListener(stop: () => void) {
    // 原生订阅异步建立；卸载后的迟到结果也必须立即释放。
    if (disposed) stop();
    else listeners.push(stop);
  }
  onMounted(async () => {
    try {
      retainListener(
        await desktop.onAppExitRequested(() => {
          exitingApp = true;
          void closeApplication();
        }),
      );
      if (disposed) return;
      retainListener(
        await desktop.onCloseRequested((event) => {
          if (closingWindow) return;
          event.preventDefault();
          void closeApplication();
        }),
      );
      if (disposed) return;
      presenceSync = setInterval(() => {
        if (!options.vaultReady.value) return;
        void desktop
          .syncSessionPresence({
            protocolVersion: 1,
            sessions: sessionPresenceSnapshot(),
          })
          .catch((cause) => options.reportError(errorMessage(cause)));
      }, 15000);
    } catch (cause) {
      if (!disposed) options.reportError(errorMessage(cause));
    }
  });
  onBeforeUnmount(() => {
    disposed = true;
    if (presenceSync) clearInterval(presenceSync);
    listeners.forEach((stop) => stop());
  });
  return { savingLogs };
}

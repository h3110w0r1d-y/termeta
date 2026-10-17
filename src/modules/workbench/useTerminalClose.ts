import { terminalBaseTitle } from "../terminal/title";
import { t } from "../../i18n";
import { nextTick } from "vue";
import { confirmAction } from "../../components/ui/confirmation";
import { errorMessage } from "../../services/desktop/client";
import { useSettings } from "../settings/store";
import { needsCloseConfirmation } from "../terminal/activity";
import { terminalRegistry } from "../terminal/registry";
import { terminalIds } from "./layout";
import { useWorkbenchStore } from "./store";

export function useTerminalClose() {
  const workbench = useWorkbenchStore();
  const settings = useSettings();
  async function closeTerminals(ids: string[], title: string) {
    if (ids.some((id) => workbench.closingTerminalIds.includes(id))) return;
    const terminals = workbench.terminals.filter((terminal) =>
      ids.includes(terminal.terminalId),
    );
    if (!terminals.length) return;
    workbench.closingTerminalIds.push(...ids);
    const restoreTabFocus = !!document.activeElement?.closest(
      "[data-app-tab], .app-tab-close",
    );
    try {
      if (
        terminals.some((terminal) => terminal.sftpTransferring) &&
        !(await confirmAction(
          t("此终端有 SFTP 传输正在进行，关闭将取消传输，确定继续？"),
        ))
      )
        return;
      const running = terminals.filter(needsCloseConfirmation);
      if (
        running.length > 0 &&
        settings.value.confirmClose &&
        !(await confirmAction(
          terminals.length > 1
            ? t(
                "“{title}”中有 {count} 个终端可能正在执行命令。关闭将断开连接并清除终端历史，是否继续？",
                { title, count: running.length },
              )
            : t(
                "“{title}”可能正在执行命令。关闭将断开连接并清除终端历史，是否继续？",
                { title },
              ),
        ))
      )
        return;
      // 每个终端成功释放后才从布局删除，失败的窗格与历史保留以便重试。
      await Promise.all(
        terminals.map(async (terminal) => {
          try {
            await terminalRegistry.dispose(terminal.terminalId);
            workbench.dismiss(terminal.sessionId ?? "");
            workbench.remove(terminal.terminalId);
          } catch (error) {
            terminal.error = errorMessage(error);
          }
        }),
      );
      await nextTick();
      if (restoreTabFocus)
        document
          .querySelector<HTMLElement>('[data-app-tab][aria-selected="true"]')
          ?.focus({ preventScroll: true });
      else if (workbench.activeTerminal)
        terminalRegistry.focus(workbench.activeTerminal.terminalId);
    } finally {
      workbench.closingTerminalIds = workbench.closingTerminalIds.filter(
        (id) => !ids.includes(id),
      );
    }
  }
  function closeTab(id: string) {
    const group = workbench.groups.find((item) => item.id === id);
    if (group) {
      const focused = workbench.terminals.find(
        (terminal) => terminal.terminalId === group.focusedTerminalId,
      );
      return closeTerminals(
        terminalIds(group.root),
        group.root.kind === "split" || group.savedProfileId
          ? (group.name ?? t("工作区"))
          : focused
            ? terminalBaseTitle(focused)
            : t("终端"),
      );
    }
  }
  function closePane(id: string) {
    const terminal = workbench.terminals.find(
      (terminal) => terminal.terminalId === id,
    );
    if (terminal) return closeTerminals([id], terminalBaseTitle(terminal));
  }
  return { closeTab, closePane };
}

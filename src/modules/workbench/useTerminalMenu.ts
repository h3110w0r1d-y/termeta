import { terminalBaseTitle } from "../terminal/title";
import { t } from "@/i18n";
import { computed, nextTick, ref } from "vue";
import { confirmAction } from "@/components/ui/confirmation";
import { useSettings } from "../settings/store";
import { builtinSchemes as builtinThemeOptions } from "@/theme/schemes";
import { terminalRegistry } from "../terminal/registry";
import { useWorkbenchStore } from "./store";
import { isSessionTerminal } from "../sessions/control";
import "./terminalMenu.css";

export function useTerminalMenu(options: {
  perform: (action: () => void | Promise<void>) => void;
  saveWorkspace: (id: string) => void;
  closeTab: (id: string) => void | Promise<void>;
  focusTerminal: (id: string) => Promise<void>;
  openFind: () => void;
}) {
  const workbench = useWorkbenchStore();
  const settings = useSettings();
  const { perform, closeTab, focusTerminal } = options;
  const tabMenu = ref({ show: false, id: "", groupId: "", x: 0, y: 0 });
  // 同时约束主菜单和配色子菜单，滚动与边缘翻转交给Naive UI处理。
  const terminalMenuProps = () => ({
    "data-ui-popup-open": tabMenu.value.show ? "" : undefined,
    class: "terminal-context-menu",
    style:
      "max-height: min(420px, calc(100dvh - 24px)); max-width: min(320px, calc(100dvw - 24px))",
  });
  const menuTerminal = computed(() =>
    workbench.terminals.find(
      (terminal) => terminal.terminalId === tabMenu.value.id,
    ),
  );
  const menuIsSingleTerminal = computed(
    () =>
      workbench.groupForTerminal(tabMenu.value.id)?.root.kind === "terminal",
  );
  const tabMenuOptions = computed(() =>
    tabMenu.value.groupId
      ? [{ label: t("workspaces.actions.saveMenu"), key: "save-workspace" }]
      : [
          {
            label: t("terminal.accessibility.title", {
              name: menuTerminal.value
                ? terminalBaseTitle(menuTerminal.value)
                : "",
            }),
            key: "target",
            disabled: true,
          },
          ...(menuIsSingleTerminal.value
            ? [
                {
                  label: t("workbench.actions.duplicateTab"),
                  key: "duplicate",
                  disabled: !workbench.canDuplicateSession(tabMenu.value.id),
                },
                {
                  label: t("workbench.actions.split"),
                  key: "split",
                  disabled: !workbench.canDuplicateSession(
                    tabMenu.value.id,
                    true,
                  ),
                },
              ]
            : []),
          { label: t("terminal.search.history"), key: "find" },
          {
            label: t("terminal.form.theme"),
            key: "theme",
            children: [
              { label: t("terminal.theme.hostDefault"), key: "scheme:default" },
              ...[...builtinThemeOptions, ...settings.value.customSchemes].map(
                (scheme) => ({
                  label: scheme.name,
                  key: `scheme:${scheme.id}`,
                }),
              ),
            ],
          },
          !isSessionTerminal(menuTerminal.value?.state ?? "closed")
            ? { label: t("terminal.actions.disconnect"), key: "disconnect" }
            : {
                label: menuTerminal.value?.sshCommand
                  ? t("terminal.actions.rerun")
                  : t("terminal.actions.reconnect"),
                key: "reconnect",
              },
        ],
  );
  function openTabMenu(event: MouseEvent | KeyboardEvent, id: string) {
    event.preventDefault();
    const group = workbench.groups.find((item) => item.id === id);
    if (!group) return;
    openTerminalMenu(event, group.focusedTerminalId);
    if (group.root.kind === "split" || group.savedProfileId)
      tabMenu.value.groupId = group.id;
  }
  function duplicateTerminal(id: string, split: boolean) {
    if (!workbench.canDuplicateSession(id, split)) return;
    workbench.duplicateSession(id, split);
    perform(() => focusTerminal(workbench.activeId));
  }
  function doubleClickTab(id: string) {
    const action = settings.value.tabDoubleClickAction;
    if (action === "close") perform(() => closeTab(id));
    else if (action === "duplicate" || action === "split") {
      const group = workbench.groups.find((item) => item.id === id);
      if (group?.root.kind === "terminal" && !group.savedProfileId)
        duplicateTerminal(group.root.terminalId, action === "split");
    }
  }
  function openTerminalMenu(event: MouseEvent | KeyboardEvent, id: string) {
    event.preventDefault();
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    tabMenu.value = {
      show: true,
      id,
      groupId: "",
      x: event instanceof MouseEvent ? event.clientX : rect.left,
      y: event instanceof MouseEvent ? event.clientY : rect.bottom,
    };
  }
  async function selectTabAction(key: string) {
    tabMenu.value.show = false;
    if (key === "save-workspace") {
      options.saveWorkspace(tabMenu.value.groupId);
      return;
    }
    const terminal = menuTerminal.value;
    if (!terminal) return;
    if (key === "duplicate" || key === "split") {
      duplicateTerminal(terminal.terminalId, key === "split");
    } else if (key === "disconnect") {
      if (
        terminal.sftpTransferring &&
        !(await confirmAction(t("files.confirmDisconnect")))
      )
        return;
      perform(() => terminalRegistry.disconnect(terminal.terminalId));
    } else if (key === "reconnect")
      perform(() => terminalRegistry.connect(terminal));
    else if (key === "find") {
      workbench.focusTerminal(terminal.terminalId);
      await nextTick();
      options.openFind();
    } else if (key.startsWith("scheme:")) {
      workbench.setTerminalScheme(
        terminal.terminalId,
        key === "scheme:default" ? undefined : key.slice(7),
      );
    }
  }
  return {
    tabMenu,
    terminalMenuProps,
    tabMenuOptions,
    openTabMenu,
    openTerminalMenu,
    doubleClickTab,
    selectTabAction,
  };
}

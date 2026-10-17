import { terminalBaseTitle } from "../terminal/title";
import { t } from "../../i18n";
import { computed, nextTick, ref } from "vue";
import { confirmAction } from "../../components/ui/confirmation";
import { useSettings } from "../settings/store";
import { builtinSchemes as builtinThemeOptions } from "../settings/themes";
import { terminalRegistry } from "../terminal/registry";
import { useWorkspace, isActive } from "./store";

export function useTerminalMenu(options: {
  perform: (action: () => void | Promise<void>) => void;
  saveWorkspace: (id: string) => void;
  closeTab: (id: string) => void | Promise<void>;
  focusTerminal: (id: string) => Promise<void>;
  openFind: () => void;
}) {
  const workspace = useWorkspace();
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
  const menuTab = computed(() =>
    workspace.tabs.find((tab) => tab.terminalId === tabMenu.value.id),
  );
  const menuIsSingleTerminal = computed(
    () =>
      workspace.groupForTerminal(tabMenu.value.id)?.root.kind === "terminal",
  );
  const tabMenuOptions = computed(() =>
    tabMenu.value.groupId
      ? [{ label: t("保存工作区…"), key: "save-workspace" }]
      : [
          {
            label: t("终端：{name}", {
              name: menuTab.value ? terminalBaseTitle(menuTab.value) : "",
            }),
            key: "target",
            disabled: true,
          },
          ...(menuIsSingleTerminal.value
            ? [
                {
                  label: t("复制标签页"),
                  key: "duplicate",
                  disabled: !workspace.canDuplicateSession(tabMenu.value.id),
                },
                {
                  label: t("拆分会话"),
                  key: "split",
                  disabled: !workspace.canDuplicateSession(
                    tabMenu.value.id,
                    true,
                  ),
                },
              ]
            : []),
          { label: t("搜索历史"), key: "find" },
          {
            label: t("终端配色"),
            key: "theme",
            children: [
              { label: t("跟随主机 / 默认配色"), key: "scheme:default" },
              ...[...builtinThemeOptions, ...settings.value.customSchemes].map(
                (scheme) => ({
                  label: scheme.name,
                  key: `scheme:${scheme.id}`,
                }),
              ),
            ],
          },
          isActive(menuTab.value?.state ?? "closed")
            ? { label: t("断开"), key: "disconnect" }
            : {
                label: menuTab.value?.sshCommand
                  ? t("重新运行")
                  : t("重新连接"),
                key: "reconnect",
              },
        ],
  );
  function openTabMenu(event: MouseEvent | KeyboardEvent, id: string) {
    event.preventDefault();
    const group = workspace.groups.find((item) => item.id === id);
    if (!group) return;
    openTerminalMenu(event, group.focusedTerminalId);
    if (group.root.kind === "split" || group.savedProfileId)
      tabMenu.value.groupId = group.id;
  }
  function duplicateTerminal(id: string, split: boolean) {
    if (!workspace.canDuplicateSession(id, split)) return;
    workspace.duplicateSession(id, split);
    perform(() => focusTerminal(workspace.activeId));
  }
  function doubleClickTab(id: string) {
    const action = settings.value.tabDoubleClickAction;
    if (action === "close") perform(() => closeTab(id));
    else if (action === "duplicate" || action === "split") {
      const group = workspace.groups.find((item) => item.id === id);
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
    const tab = menuTab.value;
    if (!tab) return;
    if (key === "duplicate" || key === "split") {
      duplicateTerminal(tab.terminalId, key === "split");
    } else if (key === "disconnect") {
      if (
        tab.sftpTransferring &&
        !(await confirmAction(t("断开 SSH 将取消当前 SFTP 传输，确定继续？")))
      )
        return;
      perform(() => terminalRegistry.disconnect(tab.terminalId));
    } else if (key === "reconnect")
      perform(() => terminalRegistry.connect(tab));
    else if (key === "find") {
      workspace.focusTerminal(tab.terminalId);
      await nextTick();
      options.openFind();
    } else if (key.startsWith("scheme:")) {
      tab.schemeId = key === "scheme:default" ? undefined : key.slice(7);
      terminalRegistry.apply();
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

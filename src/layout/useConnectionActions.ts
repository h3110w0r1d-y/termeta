import { ref, type Ref } from "vue";
import { errorMessage } from "../services/desktop/client";
import type {
  HostKind,
  HostProfile,
  OpenTarget,
  SerialConfig,
} from "../services/desktop/types";
import {
  useWorkbenchStore,
  type TerminalOpenMode,
} from "../modules/workbench/store";

/** 连接入口与编辑弹窗共享状态，创建会话仍由工作台负责。 */
export function useConnectionActions(
  savingLogs: Ref<boolean>,
  reportError: (message: string) => void,
) {
  const workbench = useWorkbenchStore();
  const newTabOpen = ref(false);
  const serialOpen = ref(false);
  const serialSave = ref(false);
  const editorKind = ref<"ssh" | "telnet">("ssh");
  const editorOpen = ref(false);
  const quick = ref(false);
  const editing = ref<HostProfile>();
  const duplicate = ref(false);
  const newHostGroup = ref<string | null>(null);

  function edit(
    host?: HostProfile,
    copy = false,
    groupId: string | null = null,
  ) {
    if (host?.kind === "workspace") return;
    newHostGroup.value = groupId;
    editing.value = host;
    duplicate.value = copy;
    quick.value = false;
    if (host?.kind === "serial") {
      serialSave.value = true;
      serialOpen.value = true;
    } else {
      editorKind.value = host?.kind === "telnet" ? "telnet" : "ssh";
      editorOpen.value = true;
    }
  }
  function createHost(kind: HostKind, groupId: string | null) {
    if (kind !== "serial") {
      edit(undefined, false, groupId);
      editorKind.value = kind;
    } else {
      editing.value = undefined;
      duplicate.value = false;
      newHostGroup.value = groupId;
      serialSave.value = true;
      serialOpen.value = true;
    }
  }
  function openNewTab() {
    if (savingLogs.value) return;
    newTabOpen.value = true;
  }
  function quickConnect(kind: HostKind = "ssh") {
    if (savingLogs.value) return;
    newHostGroup.value = null;
    editing.value = undefined;
    quick.value = true;
    duplicate.value = false;
    if (kind === "serial") {
      serialSave.value = false;
      serialOpen.value = true;
    } else {
      editorKind.value = kind;
      editorOpen.value = true;
    }
  }
  function openLocalTerminal() {
    if (!savingLogs.value) workbench.add("本地终端", { source: "local" });
  }
  function openSerial(config: SerialConfig) {
    if (!savingLogs.value)
      workbench.add(config.path, { source: "serial", config });
  }
  function connect(host: HostProfile) {
    connectMany([host], "tabs");
  }
  function connectMany(profiles: HostProfile[], mode: TerminalOpenMode) {
    if (savingLogs.value) return;
    try {
      workbench.addSavedHosts(profiles, mode);
    } catch (cause) {
      reportError(errorMessage(cause));
    }
  }
  function ephemeral(target: OpenTarget, title: string) {
    if (!savingLogs.value) workbench.add(title, target);
  }

  return {
    newTabOpen,
    serialOpen,
    serialSave,
    editorKind,
    editorOpen,
    quick,
    editing,
    duplicate,
    newHostGroup,
    edit,
    createHost,
    openNewTab,
    quickConnect,
    openLocalTerminal,
    openSerial,
    connect,
    connectMany,
    ephemeral,
  };
}

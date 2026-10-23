import { t, type PlainMessageKey } from "@/i18n";
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { useHosts } from "../hosts/store";
import { clone } from "@/services/data/clone";
import { terminalBaseTitle, terminalDisplayTitle } from "../terminal/title";
import { moveTabBefore } from "./tabInsertion";
import { broadcastTargets, type InputKind } from "../terminal/inputRouter";
import { useSettings } from "../settings/store";
import { resolveTerminalScheme } from "@/theme/schemes";
import {
  terminalIds,
  balancedLayout,
  removeTerminal,
  setSplitRatio,
  swapSplit,
  relocateTerminal,
  type LayoutNode,
  type SplitEdge,
} from "./layout";
import type {
  OpenTarget,
  SessionState,
  SessionLogSummary,
} from "@/services/desktop/types";
import type {
  HostProfile,
  ConnectionProfile,
  WorkspaceProfile,
  SavedWorkspace,
} from "@/modules/hosts/model";
export type {
  TerminalOpenMode,
  TerminalItem,
  TerminalTabGroup,
  TabBarItem,
  SessionLogTab,
  TabDragSource,
  TabDropTarget,
  TabBarDropResult,
  VaultSection,
} from "./types";
import type {
  TerminalOpenMode,
  TerminalItem,
  TerminalTabGroup,
  TabBarItem,
  SessionLogTab,
  TabDragSource,
  TabDropTarget,
  TabBarDropResult,
  VaultSection,
} from "./types";
import {
  validateSavedWorkspace,
  restoreWorkspaceLayout,
  captureWorkspace,
} from "./workspace";

export const useWorkbenchStore = defineStore("workbench", () => {
  const sidebarOpen = ref(false);
  // terminals 保存终端元数据，groups 独立保存标签顺序与布局；移动布局不增删终端。
  const terminals = ref<TerminalItem[]>([]);
  const groups = ref<TerminalTabGroup[]>([]);
  const logTabs = ref<SessionLogTab[]>([]);
  const activeId = ref("home");
  const vaultSection = ref<VaultSection>("hosts");
  const fileTransferring = ref(false);
  const closingTerminalIds = ref<string[]>([]);
  const drag = ref<{ source: TabDragSource; x: number; y: number }>();
  const dropTarget = ref<TabDropTarget>();
  const activeGroup = computed(() =>
    groups.value.find((g) => g.id === activeId.value),
  );
  const activeTerminal = computed(() =>
    terminals.value.find(
      (t) => t.terminalId === activeGroup.value?.focusedTerminalId,
    ),
  );
  const activeLogTab = computed(() =>
    logTabs.value.find((tab) => tab.id === activeId.value),
  );
  const tabIds = computed(() => [
    "home",
    "sftp",
    ...groups.value.map((g) => g.id),
    ...logTabs.value.map((tab) => tab.id),
  ]);
  const tabBarItems = computed<TabBarItem[]>(() => [
    ...groups.value.map((group): TabBarItem => {
      const ids = terminalIds(group.root);
      const terminal = terminals.value.find(
        (t) => t.terminalId === group.focusedTerminalId,
      );
      return {
        kind: "terminal",
        id: group.id,
        title: terminalDisplayTitle(
          group.root.kind === "split" || group.savedProfileId
            ? (group.name ?? t("workspaces.title"))
            : terminal
              ? terminalBaseTitle(terminal)
              : t("terminal.titles.terminal"),
          terminal?.remoteTitle,
        ),
        // 标签状态概括全部子终端，避免聚焦已关闭窗格时掩盖其他活动连接。
        state: ids.some((id) =>
          terminals.value.some(
            (t) => t.terminalId === id && t.state === "connected",
          ),
        )
          ? "connected"
          : (terminal?.state ?? "closed"),
        terminalIds: ids,
        split: group.root.kind === "split" || !!group.savedProfileId,
      };
    }),
    ...logTabs.value.map((tab): TabBarItem => ({
      kind: "log",
      id: tab.id,
      title: t("sessionLogs.tabTitle", { name: tab.log.name }),
      state: "closed",
      terminalIds: [],
      split: false,
    })),
  ]);
  function openSessionLog(log: SessionLogSummary) {
    const id = `log:${log.id}`;
    if (!logTabs.value.some((tab) => tab.id === id))
      logTabs.value.push({ id, log: clone(log) });
    activeId.value = id;
  }
  function closeSessionLogTab(id: string) {
    const at = logTabs.value.findIndex((tab) => tab.id === id);
    if (at < 0) return false;
    logTabs.value.splice(at, 1);
    if (activeId.value === id) {
      const next = logTabs.value[Math.min(at, logTabs.value.length - 1)];
      if (next) activateTab(next.id);
      else openVault("logs");
    }
    return true;
  }
  function closeSessionLog(logId: string) {
    closeSessionLogTab(`log:${logId}`);
  }
  function closeAllSessionLogs() {
    const wasLog = !!activeLogTab.value;
    logTabs.value = [];
    if (wasLog) openVault("logs");
  }
  // 序号按连接身份递增，不依赖可变的显示名称，也不将密码纳入键。
  const connectionNumbers = new Map<string, number>();
  function connectionKey(target: OpenTarget) {
    if (target.source === "local") return "local";
    if (target.source === "telnet")
      return JSON.stringify([
        "telnet",
        target.config.address.trim().toLowerCase(),
        target.config.port,
      ]);
    if (target.source === "serial")
      return JSON.stringify(["serial", target.config.path]);
    return target.source === "saved"
      ? JSON.stringify(["saved", target.profileId])
      : JSON.stringify([
          "ephemeral",
          target.kind,
          target.config.address.trim().toLowerCase(),
          target.config.port,
          target.config.username,
        ]);
  }
  function add(
    title: string,
    target: OpenTarget,
    metadata?: Pick<TerminalItem, "kind" | "serialConfig">,
  ) {
    const key = connectionKey(target);
    const number = (connectionNumbers.get(key) ?? 0) + 1;
    connectionNumbers.set(key, number);
    const terminal: TerminalItem = {
      terminalId: crypto.randomUUID(),
      title: number === 1 ? title : `${title} (${number})`,
      baseTitle: title,
      target,
      // 保存目标仍只传profileId给Rust；界面元数据独立于IPC，避免串口误用SSH能力。
      kind:
        metadata?.kind ??
        (target.source === "local"
          ? "local"
          : target.source === "serial"
            ? "serial"
            : target.source === "telnet"
              ? "telnet"
              : "ssh"),
      serialConfig:
        target.source === "serial"
          ? clone(target.config)
          : metadata?.serialConfig
            ? clone(metadata.serialConfig)
            : undefined,
      state: "created",
      revision: 0,
      error: "",
      size: { cols: 80, rows: 24 },
    };
    terminals.value.push(terminal);
    groups.value.push({
      id: terminal.terminalId,
      root: { kind: "terminal", terminalId: terminal.terminalId },
      focusedTerminalId: terminal.terminalId,
      broadcast: false,
      broadcastRevision: 0,
    });
    activeId.value = terminal.terminalId;
    return terminals.value[terminals.value.length - 1]!;
  }
  function addSavedHosts(
    profiles: HostProfile[],
    mode: TerminalOpenMode = "tabs",
  ) {
    // 同一批每个保存配置只创建一次；不复制任何已有会话或终端历史。
    const unique = [
      ...new Map(profiles.map((host) => [host.id, host])).values(),
    ];
    // 保存工作区独立恢复，不能作为单个后端终端或嵌套引用。
    const savedWorkspaces = unique.filter((host) => host.kind === "workspace");
    for (const host of savedWorkspaces)
      validateSavedWorkspace(host, useHosts().hosts);
    const created = unique
      .filter((host): host is ConnectionProfile => host.kind !== "workspace")
      .map((host) =>
        add(
          host.name,
          { source: "saved", profileId: host.id },
          {
            kind: host.kind,
            serialConfig:
              host.kind === "serial" ? host.serialConfig : undefined,
          },
        ),
      );
    const first = created[0];
    if (!first) {
      savedWorkspaces.forEach(openSavedWorkspace);
      return;
    }
    if (mode === "workspace" && created.length > 1) {
      const ids = new Set(created.map((terminal) => terminal.terminalId));
      const group = groups.value.find((item) => item.id === first.terminalId)!;
      group.root = balancedLayout([...ids])!;
      unifyGroupScheme(group, first);
      // Vue提交前收拢本批新标签，既有标签、宿主与广播成员均不受影响。
      groups.value = groups.value.filter(
        (item) => item === group || !ids.has(item.id),
      );
    }
    savedWorkspaces.forEach(openSavedWorkspace);
    activeId.value = first.terminalId;
  }
  function openSavedWorkspace(host: WorkspaceProfile) {
    const workspace = validateSavedWorkspace(host, useHosts().hosts);
    const created: TerminalItem[] = [];
    const root = restoreWorkspaceLayout(workspace.root, (profileId) => {
      const profile = useHosts().hosts.find((h) => h.id === profileId);
      if (!profile || profile.kind === "workspace")
        throw new Error(t("workspaces.errors.invalidConnection"));
      const terminal = add(
        profile.name,
        { source: "saved", profileId: profile.id },
        {
          kind: profile.kind,
          serialConfig:
            profile.kind === "serial" ? profile.serialConfig : undefined,
        },
      );
      created.push(terminal);
      return terminal.terminalId;
    });
    const ids = new Set(created.map((t) => t.terminalId));
    const group = groups.value.find((g) => g.id === created[0]!.terminalId)!;
    group.root = root;
    group.name = host.name;
    group.savedProfileId = host.id;
    group.focusedTerminalId = created[workspace.focusedIndex]!.terminalId;
    unifyGroupScheme(group, created[workspace.focusedIndex]!);
    groups.value = groups.value.filter((g) => g === group || !ids.has(g.id));
    activeId.value = group.id;
  }
  function snapshotWorkspace(id: string): SavedWorkspace {
    const group = groups.value.find((g) => g.id === id);
    if (!group || (group.root.kind !== "split" && !group.savedProfileId))
      throw new Error(t("workspaces.errors.workspaceOnly"));
    return captureWorkspace(group, terminals.value, useHosts().hosts);
  }

  function groupForTerminal(id: string) {
    return groups.value.find((g) => terminalIds(g.root).includes(id));
  }
  function unifyGroupScheme(group: TerminalTabGroup, source: TerminalItem) {
    // 固定本次实际生效的方案，避免成员的主机覆盖不同而在切换焦点时变色。
    const schemeId = resolveTerminalScheme(source, useSettings().value).id;
    const ids = new Set(terminalIds(group.root));
    for (const terminal of terminals.value)
      if (ids.has(terminal.terminalId)) terminal.schemeId = schemeId;
  }
  function setTerminalScheme(id: string, schemeId?: string) {
    const terminal = terminals.value.find((item) => item.terminalId === id);
    const group = groupForTerminal(id);
    if (!terminal || !group) return;
    if (group.root.kind === "split" || group.savedProfileId) {
      // “主机默认”取被操作成员的默认配色，再统一到整组，而非逐个清除覆盖。
      unifyGroupScheme(group, { ...terminal, schemeId });
    } else terminal.schemeId = schemeId;
  }
  function canDuplicateSession(id: string, split = false) {
    const terminal = terminals.value.find((item) => item.terminalId === id);
    return !!(
      terminal &&
      terminal.kind !== "serial" &&
      groupForTerminal(id)?.root.kind === "terminal" &&
      !closingTerminalIds.value.includes(id) &&
      !drag.value &&
      (!split || !terminal.sftpOpen)
    );
  }
  function duplicateSession(id: string, split = false) {
    if (!canDuplicateSession(id, split)) return;
    const source = terminals.value.find(
      (terminal) => terminal.terminalId === id,
    )!;
    const group = groupForTerminal(id)!;
    // 仅复制连接配置与配色；新终端独立认证，不复制历史或输入。
    const created = add(source.baseTitle, clone(source.target), {
      kind: source.kind,
      serialConfig: source.serialConfig,
    });
    created.schemeId = source.schemeId;
    if (split) {
      activeId.value = group.id;
      mergeTerminal({ kind: "tab", tabId: created.terminalId }, id, "right");
    }
  }
  function focusTerminal(id: string) {
    const group = groupForTerminal(id);
    if (!group) return;
    group.focusedTerminalId = id;
    activeId.value = group.id;
  }
  function activateTab(id: string) {
    if (!tabIds.value.includes(id)) return;
    if (id === "home" && activeId.value !== "home")
      vaultSection.value = "hosts";
    activeId.value = id;
  }
  function remove(id: string) {
    const removed = terminals.value.find((t) => t.terminalId === id);
    const group = groupForTerminal(id);
    terminals.value = terminals.value.filter((t) => t.terminalId !== id);
    if (group) {
      resetBroadcast(group);
      const at = groups.value.indexOf(group);
      const root = removeTerminal(group.root, id);
      commitGroupLayout(group, root, id);
      if (!root && activeId.value === group.id)
        activateTab(
          groups.value[Math.min(at, groups.value.length - 1)]?.id ?? "home",
        );
    }
    if (removed) {
      const key = connectionKey(removed.target);
      // 同一连接全部关闭后重置序号；布局移动与中途关闭不改名或复用序号。
      if (!terminals.value.some((t) => connectionKey(t.target) === key))
        connectionNumbers.delete(key);
    }
    cancelDrag();
  }
  function dragTerminal(source: TabDragSource) {
    if (source.kind === "pane") {
      const group = groupForTerminal(source.terminalId);
      return group && group.root.kind === "split"
        ? { group, terminalId: source.terminalId }
        : undefined;
    }
    const group = groups.value.find((g) => g.id === source.tabId);
    return group?.root.kind === "terminal"
      ? { group, terminalId: group.root.terminalId }
      : undefined;
  }
  function groupIsClosing(group: TerminalTabGroup) {
    return terminalIds(group.root).some((id) =>
      closingTerminalIds.value.includes(id),
    );
  }
  function canSplit(source: TabDragSource, targetTerminalId: string) {
    const incoming = dragTerminal(source);
    const target = activeGroup.value;
    return !!(
      incoming &&
      target &&
      incoming.terminalId !== targetTerminalId &&
      (source.kind === "pane" || incoming.group.id !== target.id) &&
      terminalIds(target.root).includes(targetTerminalId) &&
      !terminals.value.some(
        (terminal) =>
          terminal.sftpOpen &&
          (terminal.terminalId === incoming.terminalId ||
            terminalIds(target.root).includes(terminal.terminalId)),
      ) &&
      !groupIsClosing(incoming.group) &&
      !groupIsClosing(target)
    );
  }
  // 布局变更统一修复来源组焦点与空组；不删除终端元数据或重置连接编号。
  function commitGroupLayout(
    group: TerminalTabGroup,
    root: LayoutNode | undefined,
    removedTerminalId: string,
  ) {
    if (root) {
      group.root = root;
      if (group.focusedTerminalId === removedTerminalId)
        group.focusedTerminalId = terminalIds(root)[0]!;
    } else groups.value = groups.value.filter((g) => g.id !== group.id);
  }
  function mergeTerminal(
    source: TabDragSource,
    targetTerminalId: string,
    edge: SplitEdge,
  ) {
    if (!canSplit(source, targetTerminalId)) return false;
    const incoming = dragTerminal(source)!;
    const target = activeGroup.value!;
    const targetTerminal = terminals.value.find(
      (terminal) => terminal.terminalId === targetTerminalId,
    );
    if (!targetTerminal) return false;
    const moved = relocateTerminal(
      incoming.group.root,
      target.root,
      incoming.terminalId,
      targetTerminalId,
      edge,
    );
    if (!moved) return false;
    resetBroadcast(incoming.group);
    if (incoming.group !== target) resetBroadcast(target);
    // 两棵树先计算后提交；同组移动使用同一结果，始终保持叶子归属唯一。
    commitGroupLayout(incoming.group, moved.sourceRoot, incoming.terminalId);
    target.root = moved.targetRoot;
    unifyGroupScheme(target, targetTerminal);
    target.focusedTerminalId = incoming.terminalId;
    activeId.value = target.id;
    cancelDrag();
    return true;
  }
  function dropOnTabBar(
    source: TabDragSource,
    beforeGroupId: string | null,
  ): TabBarDropResult {
    if (
      beforeGroupId !== null &&
      !groups.value.some((group) => group.id === beforeGroupId)
    )
      return "invalid";
    if (source.kind === "tab") {
      const group = groups.value.find((group) => group.id === source.tabId);
      if (!group || groupIsClosing(group)) return "invalid";
      const next = moveTabBefore(groups.value, source.tabId, beforeGroupId);
      if (next === groups.value) return "unchanged";
      groups.value = next;
      return "reordered";
    }
    const incoming = dragTerminal(source);
    if (!incoming || groupIsClosing(incoming.group)) return "invalid";
    // 标签组ID不能沿用terminalId：原组可能仍以被拖出终端的ID为组ID。
    const group: TerminalTabGroup = {
      id: crypto.randomUUID(),
      root: { kind: "terminal", terminalId: incoming.terminalId },
      focusedTerminalId: incoming.terminalId,
      broadcast: false,
      broadcastRevision: 0,
    };
    resetBroadcast(incoming.group);
    commitGroupLayout(
      incoming.group,
      removeTerminal(incoming.group.root, incoming.terminalId),
      incoming.terminalId,
    );
    const index =
      beforeGroupId === null
        ? groups.value.length
        : groups.value.findIndex((group) => group.id === beforeGroupId);
    groups.value.splice(index, 0, group);
    activeId.value = group.id;
    return "detached";
  }
  // 成员变化必须显式重新开启广播；版本号也使等待粘贴确认的旧目标失效。
  function resetBroadcast(group: TerminalTabGroup) {
    group.broadcast = false;
    group.broadcastRevision++;
  }
  function toggleBroadcast() {
    const group = activeGroup.value;
    if (
      !group ||
      group.root.kind !== "split" ||
      drag.value ||
      groupIsClosing(group) ||
      activeTerminal.value?.sshCommand ||
      (!group.broadcast && broadcastTargetCount(group.focusedTerminalId) < 2)
    )
      return;
    group.broadcast = !group.broadcast;
    group.broadcastRevision++;
  }
  function inputTargets(id: string, kind: InputKind = "keyboard") {
    const group = activeGroup.value;
    if (!group?.broadcast || drag.value || groupIsClosing(group)) return [id];
    const ids = new Set(terminalIds(group.root));
    return broadcastTargets(
      id,
      terminals.value.filter((terminal) => ids.has(terminal.terminalId)),
      group.focusedTerminalId,
      kind,
    );
  }
  function broadcastTargetCount(id: string) {
    const group = groupForTerminal(id);
    if (!group) return 0;
    const ids = terminalIds(group.root);
    return terminals.value.filter(
      (terminal) =>
        ids.includes(terminal.terminalId) &&
        !terminal.sshCommand &&
        terminal.state === "connected" &&
        !closingTerminalIds.value.includes(terminal.terminalId),
    ).length;
  }
  function toggleSftp() {
    const terminal = activeTerminal.value;
    if (
      !terminal ||
      activeGroup.value?.root.kind !== "terminal" ||
      drag.value ||
      closingTerminalIds.value.includes(terminal.terminalId)
    )
      return;
    if (terminal.kind !== "ssh") return;
    if (!terminal.sftpOpen && terminal.state !== "connected") return;
    terminal.sftpOpen = !terminal.sftpOpen;
  }
  function swapSplitRegions(id: string) {
    const group = activeGroup.value;
    if (!group || drag.value || groupIsClosing(group)) return;
    group.root = swapSplit(group.root, id);
  }
  function resizeSplit(id: string, ratio: number) {
    if (activeGroup.value)
      activeGroup.value.root = setSplitRatio(activeGroup.value.root, id, ratio);
  }
  function cancelDrag() {
    drag.value = undefined;
    dropTarget.value = undefined;
  }
  function cycle(direction: number) {
    const ids = tabIds.value;
    activateTab(
      ids[(ids.indexOf(activeId.value) + direction + ids.length) % ids.length]!,
    );
  }
  function openVault(section: VaultSection = "hosts") {
    vaultSection.value = section;
    activeId.value = "home";
  }
  return {
    sidebarOpen,
    terminals,
    groups,
    logTabs,
    activeLogTab,
    openSessionLog,
    closeSessionLogTab,
    closeSessionLog,
    closeAllSessionLogs,
    tabBarItems,
    tabIds,
    activeId,
    vaultSection,
    activeGroup,
    activeTerminal,
    fileTransferring,
    closingTerminalIds,
    drag,
    dropTarget,
    add,
    addSavedHosts,
    snapshotWorkspace,
    canDuplicateSession,
    duplicateSession,
    remove,
    groupForTerminal,
    setTerminalScheme,
    focusTerminal,
    activateTab,
    dragTerminal,
    dropOnTabBar,
    canSplit,
    mergeTerminal,
    toggleBroadcast,
    broadcastTargetCount,
    toggleSftp,
    inputTargets,
    resizeSplit,
    swapSplitRegions,
    cancelDrag,
    cycle,
    openVault,
  };
});
export const stateLabels = {
  created: "terminal.state.created",
  connecting: "terminal.state.connecting",
  awaitingHostKey: "terminal.state.awaitingHostKey",
  authenticating: "terminal.state.authenticating",
  awaitingAuth: "terminal.state.awaitingAuth",
  opening: "terminal.state.opening",
  connected: "terminal.state.connected",
  closing: "terminal.state.closing",
  closed: "terminal.state.closed",
  failed: "terminal.state.failed",
  disconnected: "terminal.state.disconnected",
} as const satisfies Record<SessionState, PlainMessageKey>;
export function isActive(state: SessionState) {
  return !["closed", "failed", "disconnected"].includes(state);
}

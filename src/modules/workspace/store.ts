import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { clone } from "../../services/data/clone";
import { terminalDisplayTitle } from "../terminal/title";
import type { TerminalActivity } from "../terminal/activity";
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
  AuthChallenge,
  HostKeyChallenge,
  OpenTarget,
  SessionState,
  SessionSnapshot,
  SerialConfig,
  HostProfile,
} from "../../services/desktop/types";
export type TerminalOpenMode = "tabs" | "workspace";
export interface TerminalTab extends TerminalActivity {
  terminalId: string;
  title: string;
  baseTitle: string;
  target: OpenTarget;
  kind: SessionSnapshot["kind"];
  serialConfig?: SerialConfig;
  state: SessionState;
  sessionId?: string;
  sftpOpen?: boolean;
  sftpTransferring?: boolean;
  revision: number;
  error: string;
  schemeId?: string;
  size: { cols: number; rows: number };
}
export interface WorkspaceTab {
  id: string;
  root: LayoutNode;
  focusedTerminalId: string;
  broadcast: boolean;
  broadcastRevision: number;
}
export interface WorkspaceTabItem {
  id: string;
  title: string;
  state: SessionState;
  terminalIds: string[];
  split: boolean;
}
export type WorkspaceDragSource =
  { kind: "tab"; tabId: string } | { kind: "pane"; terminalId: string };
export type WorkspaceDropTarget =
  | { kind: "tab"; tabId: string; before: boolean }
  | { kind: "pane"; terminalId: string; edge: SplitEdge }
  | { kind: "tab-end" };
export type VaultSection = "hosts" | "credentials" | "known-hosts" | "history";
export const useWorkspace = defineStore("workspace", () => {
  // tabs 仅保存终端元数据，groups 独立保存标签顺序与布局；移动布局不增删终端。
  const tabs = ref<TerminalTab[]>([]);
  const groups = ref<WorkspaceTab[]>([]);
  const activeId = ref("home");
  const vaultSection = ref<VaultSection>("hosts");
  const fileTransferring = ref(false);
  const fileSessionTitles = ref<Record<string, string>>({});
  const challenges = ref<(AuthChallenge | HostKeyChallenge)[]>([]);
  const closingTerminalIds = ref<string[]>([]);
  const drag = ref<{ source: WorkspaceDragSource; x: number; y: number }>();
  const dropTarget = ref<WorkspaceDropTarget>();
  const activeGroup = computed(() =>
    groups.value.find((g) => g.id === activeId.value),
  );
  const active = computed(() =>
    tabs.value.find(
      (t) => t.terminalId === activeGroup.value?.focusedTerminalId,
    ),
  );
  const tabIds = computed(() => [
    "home",
    "sftp",
    ...groups.value.map((g) => g.id),
  ]);
  const workspaceTabs = computed<WorkspaceTabItem[]>(() =>
    groups.value.map((group) => {
      const ids = terminalIds(group.root);
      const terminal = tabs.value.find(
        (t) => t.terminalId === group.focusedTerminalId,
      );
      return {
        id: group.id,
        title: terminalDisplayTitle(
          group.root.kind === "split"
            ? "Workspace"
            : (terminal?.title ?? "终端"),
          terminal?.remoteTitle,
        ),
        // 标签状态概括全部子终端，避免聚焦已关闭窗格时掩盖其他活动连接。
        state: ids.some((id) =>
          tabs.value.some(
            (t) => t.terminalId === id && t.state === "connected",
          ),
        )
          ? "connected"
          : (terminal?.state ?? "closed"),
        terminalIds: ids,
        split: group.root.kind === "split",
      };
    }),
  );
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
    metadata?: Pick<TerminalTab, "kind" | "serialConfig">,
  ) {
    const key = connectionKey(target);
    const number = (connectionNumbers.get(key) ?? 0) + 1;
    connectionNumbers.set(key, number);
    const tab: TerminalTab = {
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
    tabs.value.push(tab);
    groups.value.push({
      id: tab.terminalId,
      root: { kind: "terminal", terminalId: tab.terminalId },
      focusedTerminalId: tab.terminalId,
      broadcast: false,
      broadcastRevision: 0,
    });
    activeId.value = tab.terminalId;
    return tabs.value[tabs.value.length - 1]!;
  }
  function addSavedHosts(
    profiles: HostProfile[],
    mode: TerminalOpenMode = "tabs",
  ) {
    // 同一批每个保存配置只创建一次；不复制任何已有会话或终端历史。
    const unique = [
      ...new Map(profiles.map((host) => [host.id, host])).values(),
    ];
    const created = unique.map((host) =>
      add(
        host.name,
        { source: "saved", profileId: host.id },
        {
          kind: host.kind,
          serialConfig: host.serialConfig ?? undefined,
        },
      ),
    );
    const first = created[0];
    if (!first) return;
    if (mode === "workspace" && created.length > 1) {
      const ids = new Set(created.map((tab) => tab.terminalId));
      const group = groups.value.find((item) => item.id === first.terminalId)!;
      group.root = balancedLayout([...ids])!;
      // Vue提交前收拢本批新标签，既有标签、宿主与广播成员均不受影响。
      groups.value = groups.value.filter(
        (item) => item === group || !ids.has(item.id),
      );
    }
    activeId.value = first.terminalId;
  }
  function groupForTerminal(id: string) {
    return groups.value.find((g) => terminalIds(g.root).includes(id));
  }
  function canDuplicateSession(id: string, split = false) {
    const tab = tabs.value.find((item) => item.terminalId === id);
    return !!(
      tab &&
      tab.kind !== "serial" &&
      groupForTerminal(id)?.root.kind === "terminal" &&
      !closingTerminalIds.value.includes(id) &&
      !drag.value &&
      (!split || !tab.sftpOpen)
    );
  }
  function duplicateSession(id: string, split = false) {
    if (!canDuplicateSession(id, split)) return;
    const source = tabs.value.find((tab) => tab.terminalId === id)!;
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
    const removed = tabs.value.find((t) => t.terminalId === id);
    const group = groupForTerminal(id);
    tabs.value = tabs.value.filter((t) => t.terminalId !== id);
    if (group) {
      resetBroadcast(group);
      const root = removeTerminal(group.root, id);
      if (root) {
        group.root = root;
        if (group.focusedTerminalId === id)
          group.focusedTerminalId = terminalIds(root)[0]!;
      } else {
        const at = groups.value.indexOf(group);
        groups.value = groups.value.filter((g) => g.id !== group.id);
        if (activeId.value === group.id)
          activateTab(
            groups.value[Math.min(at, groups.value.length - 1)]?.id ?? "home",
          );
      }
    }
    if (removed) {
      const key = connectionKey(removed.target);
      // 同一连接全部关闭后重置序号；布局移动与中途关闭不改名或复用序号。
      if (!tabs.value.some((t) => connectionKey(t.target) === key))
        connectionNumbers.delete(key);
    }
    cancelDrag();
  }
  function reorderTab(id: string, targetId: string, before: boolean) {
    if (id === targetId) return;
    const source = groups.value.find((g) => g.id === id);
    if (!source || !groups.value.some((g) => g.id === targetId)) return;
    const next = groups.value.filter((g) => g.id !== id);
    const at = next.findIndex((g) => g.id === targetId);
    next.splice(at + (before ? 0 : 1), 0, source);
    groups.value = next;
  }
  function dragTerminal(source: WorkspaceDragSource) {
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
  function groupIsClosing(group: WorkspaceTab) {
    return terminalIds(group.root).some((id) =>
      closingTerminalIds.value.includes(id),
    );
  }
  function canSplit(source: WorkspaceDragSource, targetTerminalId: string) {
    const incoming = dragTerminal(source);
    const target = activeGroup.value;
    return !!(
      incoming &&
      target &&
      incoming.terminalId !== targetTerminalId &&
      (source.kind === "pane" || incoming.group.id !== target.id) &&
      terminalIds(target.root).includes(targetTerminalId) &&
      !tabs.value.some(
        (tab) =>
          tab.sftpOpen &&
          (tab.terminalId === incoming.terminalId ||
            terminalIds(target.root).includes(tab.terminalId)),
      ) &&
      !groupIsClosing(incoming.group) &&
      !groupIsClosing(target)
    );
  }
  // 仅移除布局归属，不调用 remove/dispose，避免删除终端元数据或重置重复编号。
  function extractTerminal(group: WorkspaceTab, id: string) {
    resetBroadcast(group);
    const root = removeTerminal(group.root, id);
    if (root) {
      group.root = root;
      if (group.focusedTerminalId === id)
        group.focusedTerminalId = terminalIds(root)[0]!;
    } else groups.value = groups.value.filter((g) => g.id !== group.id);
  }
  function mergeTerminal(
    source: WorkspaceDragSource,
    targetTerminalId: string,
    edge: SplitEdge,
  ) {
    if (!canSplit(source, targetTerminalId)) return false;
    const incoming = dragTerminal(source)!;
    const target = activeGroup.value!;
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
    if (moved.sourceRoot) {
      incoming.group.root = moved.sourceRoot;
      if (incoming.group.focusedTerminalId === incoming.terminalId)
        incoming.group.focusedTerminalId = terminalIds(moved.sourceRoot)[0]!;
    } else
      groups.value = groups.value.filter((g) => g.id !== incoming.group.id);
    target.root = moved.targetRoot;
    target.focusedTerminalId = incoming.terminalId;
    activeId.value = target.id;
    cancelDrag();
    return true;
  }
  function dropOnTabBar(
    source: WorkspaceDragSource,
    target?: { tabId: string; before: boolean },
  ) {
    if (source.kind === "tab") {
      if (target) reorderTab(source.tabId, target.tabId, target.before);
      else {
        const last = groups.value[groups.value.length - 1];
        if (last) reorderTab(source.tabId, last.id, false);
      }
      return false;
    }
    const incoming = dragTerminal(source);
    if (
      !incoming ||
      groupIsClosing(incoming.group) ||
      (target && !groups.value.some((g) => g.id === target.tabId))
    )
      return false;
    // 标签组ID不能沿用terminalId：原组可能仍以被拖出终端的ID为组ID。
    const group: WorkspaceTab = {
      id: crypto.randomUUID(),
      root: { kind: "terminal", terminalId: incoming.terminalId },
      focusedTerminalId: incoming.terminalId,
      broadcast: false,
      broadcastRevision: 0,
    };
    extractTerminal(incoming.group, incoming.terminalId);
    const at = target
      ? groups.value.findIndex((g) => g.id === target.tabId) +
        (target.before ? 0 : 1)
      : groups.value.length;
    groups.value.splice(at, 0, group);
    activeId.value = group.id;
    cancelDrag();
    return true;
  }
  // 成员变化必须显式重新开启广播；版本号也使等待粘贴确认的旧目标失效。
  function resetBroadcast(group: WorkspaceTab) {
    group.broadcast = false;
    group.broadcastRevision++;
  }
  function toggleBroadcast() {
    const group = activeGroup.value;
    if (
      !group ||
      group.root.kind !== "split" ||
      drag.value ||
      groupIsClosing(group)
    )
      return;
    group.broadcast = !group.broadcast;
    group.broadcastRevision++;
  }
  function inputTargets(id: string) {
    const group = activeGroup.value;
    return group?.broadcast &&
      group.focusedTerminalId === id &&
      !drag.value &&
      !groupIsClosing(group)
      ? terminalIds(group.root)
      : [id];
  }
  function toggleSftp() {
    const tab = active.value;
    if (
      !tab ||
      activeGroup.value?.root.kind !== "terminal" ||
      drag.value ||
      closingTerminalIds.value.includes(tab.terminalId)
    )
      return;
    if (tab.kind !== "ssh") return;
    if (!tab.sftpOpen && tab.state !== "connected") return;
    tab.sftpOpen = !tab.sftpOpen;
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
  function dismiss(sessionId: string, challengeId?: string) {
    challenges.value = challenges.value.filter(
      (c) =>
        c.sessionId !== sessionId ||
        (challengeId !== undefined && c.challengeId !== challengeId),
    );
  }
  return {
    tabs,
    groups,
    workspaceTabs,
    tabIds,
    activeId,
    vaultSection,
    activeGroup,
    active,
    challenges,
    fileTransferring,
    fileSessionTitles,
    closingTerminalIds,
    drag,
    dropTarget,
    add,
    addSavedHosts,
    canDuplicateSession,
    duplicateSession,
    remove,
    groupForTerminal,
    focusTerminal,
    activateTab,
    reorderTab,
    dragTerminal,
    dropOnTabBar,
    canSplit,
    mergeTerminal,
    toggleBroadcast,
    toggleSftp,
    inputTargets,
    resizeSplit,
    swapSplitRegions,
    cancelDrag,
    cycle,
    openVault,
    dismiss,
  };
});
export const stateLabels: Record<SessionState, string> = {
  created: "准备连接",
  connecting: "连接中",
  awaitingHostKey: "等待主机核验",
  authenticating: "认证中",
  awaitingAuth: "等待认证",
  opening: "打开终端",
  connected: "已连接",
  closing: "正在断开",
  closed: "已关闭连接",
  failed: "连接失败",
  disconnected: "已断开",
};
export function isActive(state: SessionState) {
  return !["closed", "failed", "disconnected"].includes(state);
}

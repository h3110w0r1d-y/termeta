import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type {
  AuthChallenge,
  HostKeyChallenge,
  OpenTarget,
  SessionState,
} from "../../services/desktop/types";
export interface TerminalTab {
  terminalId: string;
  title: string;
  target: OpenTarget;
  state: SessionState;
  sessionId?: string;
  revision: number;
  error: string;
  schemeId?: string;
  size: { cols: number; rows: number };
}
export const useWorkspace = defineStore("workspace", () => {
  // 只保存视图元数据；输出、Channel 和 xterm 实例均由 TerminalRegistry 持有。
  const tabs = ref<TerminalTab[]>([]),
    activeId = ref("");
  const challenges = ref<(AuthChallenge | HostKeyChallenge)[]>([]);
  const active = computed(() =>
    tabs.value.find((t) => t.terminalId === activeId.value),
  );
  function add(title: string, target: OpenTarget) {
    const tab: TerminalTab = {
      terminalId: crypto.randomUUID(),
      title,
      target,
      state: "created",
      revision: 0,
      error: "",
      size: { cols: 80, rows: 24 },
    };
    tabs.value.push(tab);
    activeId.value = tab.terminalId;
    return tabs.value[tabs.value.length - 1]!;
  }
  function remove(id: string) {
    const at = tabs.value.findIndex((t) => t.terminalId === id);
    tabs.value = tabs.value.filter((t) => t.terminalId !== id);
    if (activeId.value === id)
      activeId.value =
        tabs.value[Math.min(at, tabs.value.length - 1)]?.terminalId ?? "";
  }
  function cycle(direction: number) {
    const n = tabs.value.length;
    if (n)
      activeId.value =
        tabs.value[
          (tabs.value.findIndex((t) => t.terminalId === activeId.value) +
            direction +
            n) %
            n
        ]!.terminalId;
  }
  function dismiss(sessionId: string) {
    challenges.value = challenges.value.filter(
      (c) => c.sessionId !== sessionId,
    );
  }
  return { tabs, activeId, active, challenges, add, remove, cycle, dismiss };
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

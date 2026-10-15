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
    activeId = ref("home");
  const homeSection = ref<"hosts" | "credentials" | "settings">("hosts");
  const tabIds = computed(() => [
    "home",
    "sftp",
    ...tabs.value.map((tab) => tab.terminalId),
  ]);
  const challenges = ref<(AuthChallenge | HostKeyChallenge)[]>([]);
  const active = computed(() =>
    tabs.value.find((t) => t.terminalId === activeId.value),
  );
  // 序号按连接身份递增，不依赖可变的显示名称，也不将密码纳入键。
  const connectionNumbers = new Map<string, number>();
  function connectionKey(target: OpenTarget) {
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
  function add(title: string, target: OpenTarget) {
    const key = connectionKey(target);
    const number = (connectionNumbers.get(key) ?? 0) + 1;
    connectionNumbers.set(key, number);
    const tab: TerminalTab = {
      terminalId: crypto.randomUUID(),
      title: number === 1 ? title : `${title} (${number})`,
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
    const removed = tabs.value.find((tab) => tab.terminalId === id);
    const at = tabs.value.findIndex((t) => t.terminalId === id);
    tabs.value = tabs.value.filter((t) => t.terminalId !== id);
    if (removed) {
      const key = connectionKey(removed.target);
      // 同一连接全部关闭后重置序号；中途关闭不改名或复用序号。
      if (!tabs.value.some((tab) => connectionKey(tab.target) === key))
        connectionNumbers.delete(key);
    }
    if (activeId.value === id)
      activeId.value =
        tabs.value[Math.min(at, tabs.value.length - 1)]?.terminalId ?? "home";
  }
  function cycle(direction: number) {
    const ids = tabIds.value;
    activeId.value =
      ids[(ids.indexOf(activeId.value) + direction + ids.length) % ids.length]!;
  }
  function openHome(section: "hosts" | "credentials" | "settings" = "hosts") {
    homeSection.value = section;
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
    tabIds,
    activeId,
    homeSection,
    active,
    challenges,
    add,
    remove,
    cycle,
    openHome,
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

import { defineStore } from "pinia";
import { ref } from "vue";
import type {
  AuthChallenge,
  HostKeyChallenge,
  SessionEvent,
  SessionPresenceKind,
  SessionSnapshot,
} from "@/services/desktop/types";
import {
  registerSessionPresence,
  unregisterSessionPresence,
} from "@/services/desktop/sessionPresence";
import {
  acceptSessionEvent,
  acceptSessionSnapshot,
  acceptsSessionChallenge,
  isSessionTerminal,
  type SessionControl,
} from "./control";

/** 三种用途共享认证队列，布局 Store 不再持有文件与转发的控制状态。 */
export const useSessions = defineStore("sessions", () => {
  const challenges = ref<(AuthChallenge | HostKeyChallenge)[]>([]);
  const titles = ref<Record<string, string>>({});
  function dismiss(sessionId: string, challengeId?: string) {
    challenges.value = challenges.value.filter(
      (challenge) =>
        challenge.sessionId !== sessionId ||
        (challengeId !== undefined && challenge.challengeId !== challengeId),
    );
  }
  function release(kind: SessionPresenceKind, sessionId: string) {
    unregisterSessionPresence(kind, sessionId);
    dismiss(sessionId);
    delete titles.value[sessionId];
  }
  function publish(
    control: SessionControl,
    kind: SessionPresenceKind,
    title: string,
  ) {
    if (!control.sessionId) return;
    if (control.ended || isSessionTerminal(control.state)) {
      release(kind, control.sessionId);
      return;
    }
    // 状态查询与命令响应也能先于 Channel 前进，过期挑战须随状态撤销。
    challenges.value = challenges.value.filter(
      (challenge) =>
        challenge.sessionId !== control.sessionId ||
        acceptsSessionChallenge(control.state, challenge.type),
    );
    registerSessionPresence(kind, control.sessionId);
    titles.value[control.sessionId] = title;
  }
  function snapshot(
    control: SessionControl,
    value: SessionSnapshot,
    kind: SessionPresenceKind,
    title = value.title,
  ) {
    if (!acceptSessionSnapshot(control, value)) return false;
    publish(control, kind, title);
    return true;
  }
  function event(
    control: SessionControl,
    value: SessionEvent,
    kind: SessionPresenceKind,
    title: string,
  ) {
    if (!acceptSessionEvent(control, value)) return false;
    publish(control, kind, title);
    if (value.type === "authChallenge" || value.type === "hostKeyChallenge") {
      dismiss(value.sessionId);
      challenges.value.push(value);
    }
    return true;
  }
  return { challenges, titles, dismiss, release, snapshot, event };
});

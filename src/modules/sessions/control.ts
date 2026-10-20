import type {
  SessionEvent,
  SessionSnapshot,
  SessionState,
} from "@/services/desktop/types";

/** 一次请求的轻量控制投影，不持有输出、凭据或连接资源。 */
export interface SessionControl {
  requestId: string;
  sessionId?: string;
  state: SessionState;
  revision: number;
  eventSeq: number;
  ended: boolean;
}

export function createSessionControl(requestId: string): SessionControl {
  return {
    requestId,
    state: "created",
    revision: -1,
    eventSeq: 0,
    ended: false,
  };
}

export function isSessionTerminal(state: SessionState) {
  return state === "closed" || state === "failed" || state === "disconnected";
}

export function acceptsSessionChallenge(
  state: SessionState,
  type: "authChallenge" | "hostKeyChallenge",
) {
  // 首个挑战可以早于初始快照；已知状态离开对应等待阶段后，不再接受迟到提示。
  return (
    state === "created" ||
    state === "connecting" ||
    state === (type === "authChallenge" ? "awaitingAuth" : "awaitingHostKey")
  );
}

export function acceptSessionSnapshot(
  control: SessionControl,
  snapshot: SessionSnapshot,
) {
  // 首个事件可以先绑定 sessionId；迟到的初始快照不能覆盖终态或串入另一请求。
  if (
    control.ended ||
    snapshot.protocolVersion !== 1 ||
    snapshot.requestId !== control.requestId ||
    typeof snapshot.sessionId !== "string" ||
    !snapshot.sessionId ||
    (control.sessionId && control.sessionId !== snapshot.sessionId) ||
    !Number.isSafeInteger(snapshot.stateRevision) ||
    snapshot.stateRevision < 0 ||
    snapshot.stateRevision <= control.revision
  )
    return false;
  control.sessionId = snapshot.sessionId;
  control.state = snapshot.state;
  control.revision = snapshot.stateRevision;
  return true;
}

export function acceptSessionEvent(
  control: SessionControl,
  event: SessionEvent,
) {
  if (
    control.ended ||
    event.protocolVersion !== 1 ||
    event.requestId !== control.requestId ||
    typeof event.sessionId !== "string" ||
    !event.sessionId ||
    (control.sessionId && control.sessionId !== event.sessionId) ||
    !Number.isSafeInteger(event.eventSeq) ||
    event.eventSeq <= control.eventSeq
  )
    return false;
  if (
    (event.type === "authChallenge" || event.type === "hostKeyChallenge") &&
    !acceptsSessionChallenge(control.state, event.type)
  ) {
    control.eventSeq = event.eventSeq;
    return false;
  }
  if (event.type === "stateChanged") {
    if (
      event.snapshot.sessionId !== event.sessionId ||
      event.snapshot.requestId !== event.requestId ||
      event.snapshot.protocolVersion !== 1
    )
      return false;
    // 查询可能已提供更高 revision；仍消费事件序号，拒绝随后重放的旧挑战。
    control.eventSeq = event.eventSeq;
    if (!acceptSessionSnapshot(control, event.snapshot)) return false;
  }
  control.sessionId = event.sessionId;
  control.eventSeq = event.eventSeq;
  if (event.type === "ended") {
    control.ended = true;
    control.state = event.finalState;
  }
  return true;
}

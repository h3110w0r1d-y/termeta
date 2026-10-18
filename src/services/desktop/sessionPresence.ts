import type { SessionPresenceKind, SessionPresenceRef } from "./types";

const sessions = new Map<string, SessionPresenceRef>();

function key(kind: SessionPresenceKind, sessionId: string) {
  return `${kind}:${sessionId}`;
}

/** 记录当前 WebView 仍持有的后端会话，供全量心跳生成对账快照。 */
export function registerSessionPresence(
  kind: SessionPresenceKind,
  sessionId: string,
) {
  sessions.set(key(kind, sessionId), { kind, sessionId });
}

export function unregisterSessionPresence(
  kind: SessionPresenceKind,
  sessionId: string,
) {
  sessions.delete(key(kind, sessionId));
}

export function sessionPresenceSnapshot(): SessionPresenceRef[] {
  return [...sessions.values()];
}

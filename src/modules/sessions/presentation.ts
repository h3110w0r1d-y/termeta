import type { PlainMessageKey } from "@/i18n";
import type { SessionState } from "@/services/desktop/types";

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

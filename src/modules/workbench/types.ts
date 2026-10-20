import type { TerminalActivity } from "../terminal/activity";
import type { LayoutNode, SplitEdge } from "./layout";
import type {
  OpenTarget,
  SessionSnapshot,
  SerialConfig,
  SessionState,
  SessionLogSummary,
} from "../../services/desktop/types";

export type TerminalOpenMode = "tabs" | "workspace";
/** 单个终端的轻量运行状态；分屏后也不对应独立标签页。 */
export interface TerminalItem extends TerminalActivity {
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
/** 一个动态标签的运行布局，可容纳一个或多个终端；工作区存档另用 SavedWorkspace。 */
export interface TerminalTabGroup {
  name?: string;
  savedProfileId?: string;
  id: string;
  root: LayoutNode;
  focusedTerminalId: string;
  broadcast: boolean;
  broadcastRevision: number;
}
/** 标签栏的展示投影，不承载终端实例或工作区存档。 */
export interface TabBarItem {
  kind: "terminal" | "log";
  id: string;
  title: string;
  state: SessionState;
  terminalIds: string[];
  split: boolean;
}
/** 日志标签只保存摘要；正文由当前回看组件按需读取。 */
export interface SessionLogTab {
  id: string;
  log: SessionLogSummary;
}
export type TabDragSource =
  { kind: "tab"; tabId: string } | { kind: "pane"; terminalId: string };
export type TabDropTarget =
  | { kind: "tab-slot"; beforeGroupId: string | null }
  | { kind: "pane"; terminalId: string; edge: SplitEdge };
export type TabBarDropResult =
  "invalid" | "unchanged" | "reordered" | "detached";
export type VaultSection =
  "hosts" | "forwarding" | "credentials" | "snippets" | "known-hosts" | "logs";

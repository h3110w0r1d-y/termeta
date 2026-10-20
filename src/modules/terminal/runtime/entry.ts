import type { Terminal } from "@xterm/xterm";
import type { FitAddon } from "@xterm/addon-fit";
import type { SearchAddon } from "@xterm/addon-search";
import type { SessionLogDraft } from "../../../services/desktop/types";
import type { observeTerminalInput } from "../inputOrigin";
import type { TerminalItem } from "../../workbench/store";

/** 每次连接独立持有字节预算、序号与解析进度，不能跨重连复用。 */
export interface Stream {
  sessionId?: string;
  seq: number;
  received: bigint;
  consumed: bigint;
  pending: number;
  acked: bigint;
  ack: Promise<void>;
  inputSeq: number;
  inputBytes: number;
  input: Promise<void>;
  ended: boolean;
  endReceived: boolean;
  finalization?: Promise<void>;
  finalOffset?: bigint;
  ackTimer?: ReturnType<typeof setTimeout>;
  eventSeq: number;
}
/** 稳定终端视图与一次性连接分离；只有关闭归档成功才销毁视图。 */
export interface Entry {
  terminal: TerminalItem;
  term: Terminal;
  fit: FitAddon;
  search: SearchAddon;
  host: HTMLElement;
  observer: ResizeObserver;
  stream?: Stream;
  connecting: boolean;
  closing?: Promise<void>;
  log: Omit<SessionLogDraft, "content">;
  disposed: boolean;
  lastSize: string;
  canResize: boolean;
  inputOrigin?: ReturnType<typeof observeTerminalInput>;
  fontRequest?: string;
}

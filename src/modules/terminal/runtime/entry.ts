import type { Terminal } from "@xterm/xterm";
import type { FitAddon } from "@xterm/addon-fit";
import type { SearchAddon } from "@xterm/addon-search";
import type { SessionLogDraft } from "@/services/desktop/types";
import type { observeTerminalInput } from "../inputOrigin";
import type { TerminalItem } from "@/modules/workbench/store";
import type { SessionControl } from "@/modules/sessions/control";

/** 每次连接独立持有字节预算、序号与解析进度，不能跨重连复用。 */
export interface Stream {
  control: SessionControl;
  seq: number;
  received: bigint;
  consumed: bigint;
  pending: number;
  acked: bigint;
  ack: Promise<void>;
  inputSeq: number;
  inputBytes: number;
  input: Promise<void>;
  inputStopped: boolean;
  finalization?: Promise<void>;
  finalOffset?: bigint;
  ackTimer?: ReturnType<typeof setTimeout>;
  whenEnded: Promise<void>;
  finish: () => void;
  progress: Set<() => void>;
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
  connection?: Promise<void>;
  closing?: Promise<void>;
  log: Omit<SessionLogDraft, "content">;
  disposed: boolean;
  lastSize: string;
  canResize: boolean;
  inputOrigin?: ReturnType<typeof observeTerminalInput>;
  fontRequest?: string;
}

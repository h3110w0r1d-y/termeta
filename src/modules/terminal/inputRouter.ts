import type { TerminalInput } from "./inputOrigin";

export type InputKind = "keyboard" | "wheel";

export interface FinalWheelInput {
  direction: "up" | "down";
  mouse: boolean;
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
}

/** 命令窗格仅接收直接输入；作为来源时也不向其他窗格扩散。 */
export function broadcastTargets(
  sourceId: string,
  members: { terminalId: string; sshCommand?: boolean }[],
  focusedId = sourceId,
  kind: InputKind = "keyboard",
): string[] {
  const source = members.find((tab) => tab.terminalId === sourceId);
  if (
    !source ||
    source.sshCommand ||
    (kind !== "wheel" && focusedId !== sourceId)
  )
    return [sourceId];
  return members.filter((tab) => !tab.sshCommand).map((tab) => tab.terminalId);
}

interface InputPorts {
  targets: (sourceId: string, kind: InputKind) => string[];
  send: (id: string, bytes: Uint8Array) => void;
  applicationCursor: (id: string) => boolean;
  wheel: (
    id: string,
    input: FinalWheelInput,
    sourceId: string,
    event?: WheelEvent,
  ) => void;
}

/** 鼠标点击、释放和移动不可广播；滚轮仅在编码作用域内解析最终上报。 */
export function isMouseReport(data: string) {
  return /^\x1b(?:\[M|\[<\d+;\d+;\d+[mM]$|\[\d+;\d+;\d+M$)/.test(data);
}
export function cursorInput(data: string, application: boolean) {
  return /^\x1b[O\[][ABCDHF]$/.test(data)
    ? "\x1b" + (application ? "O" : "[") + data.slice(-1)
    : data;
}

/** 固定 xterm 6 每次生成一个方向键或鼠标上报；仅提取实际发出的垂直滚轮动作。 */
export function decodeWheelInput(
  data: string,
  binary: boolean,
): FinalWheelInput | undefined {
  if (!binary && /^\x1b[O\[][AB]$/.test(data))
    return {
      direction: data.endsWith("A") ? "up" : "down",
      mouse: false,
      ctrl: false,
      alt: false,
      shift: false,
    };
  const sgr = !binary ? /^\x1b\[<(\d+);\d+;\d+M$/.exec(data) : null;
  const button = sgr
    ? Number(sgr[1])
    : binary && /^\x1b\[M[\s\S]{3}$/.test(data)
      ? data.charCodeAt(3) - 32
      : -1;
  if (
    !Number.isInteger(button) ||
    button < 64 ||
    button > 95 ||
    (button & 3) > 1
  )
    return undefined;
  return {
    direction: button & 1 ? "down" : "up",
    mouse: true,
    ctrl: !!(button & 16),
    alt: !!(button & 8),
    shift: !!(button & 4),
  };
}

/** 只路由输入，不保存正文；每个目标沿用自身FIFO/限额/会话代次检查。 */
export function createInputRouter(ports: InputPorts) {
  const localDepth = new Map<string, number>();
  function local(id: string, action: () => void) {
    localDepth.set(id, (localDepth.get(id) ?? 0) + 1);
    try {
      action();
    } finally {
      const depth = (localDepth.get(id) ?? 1) - 1;
      if (depth) localDepth.set(id, depth);
      else localDepth.delete(id);
    }
  }
  return {
    local,
    data(sourceId: string, packet: TerminalInput) {
      const bytes = packet.binary
        ? Uint8Array.from(packet.data, (c) => c.charCodeAt(0))
        : new TextEncoder().encode(packet.data);
      // 原终端始终只发送一次；广播生成的输入只回目标连接，不再向外扩散。
      ports.send(sourceId, bytes);
      if (localDepth.has(sourceId)) return;
      if (packet.wheel) {
        const final = decodeWheelInput(packet.data, packet.binary);
        if (!final || (!packet.userInput && !packet.binary)) return;
        // 以最终动作作为广播时钟，目标不接收 DOM wheel，也不再累积触摸板增量。
        for (const id of new Set(ports.targets(sourceId, "wheel"))) {
          if (id !== sourceId)
            local(id, () =>
              ports.wheel(id, final, sourceId, packet.wheelEvent),
            );
        }
        return;
      }
      if (!packet.userInput || packet.binary || isMouseReport(packet.data))
        return;
      for (const id of new Set(ports.targets(sourceId, "keyboard"))) {
        if (id !== sourceId) {
          const data = cursorInput(packet.data, ports.applicationCursor(id));
          ports.send(id, new TextEncoder().encode(data));
        }
      }
    },
  };
}

import type { TerminalInput } from "./inputOrigin";

interface InputPorts {
  targets: (sourceId: string) => string[];
  send: (id: string, bytes: Uint8Array) => void;
  applicationCursor: (id: string) => boolean;
  wheel: (id: string, event: WheelEvent, sourceId: string) => void;
}

/** 鼠标上报包含点击、释放、移动和滚轮；只由独立滚轮路径广播，不复制上报字节。 */
export function isMouseReport(data: string) {
  return /^\x1b(?:\[M|\[<\d+;\d+;\d+[mM]$|\[\d+;\d+;\d+M$)/.test(data);
}
export function cursorInput(data: string, application: boolean) {
  return /^\x1b[O\[][ABCDHF]$/.test(data)
    ? "\x1b" + (application ? "O" : "[") + data.slice(-1)
    : data;
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
      // 原终端始终只发送一次；设备/焦点应答和所有鼠标字节均留在原连接。
      ports.send(sourceId, bytes);
      if (
        localDepth.has(sourceId) ||
        !packet.userInput ||
        packet.binary ||
        packet.wheel ||
        isMouseReport(packet.data)
      )
        return;
      for (const id of new Set(ports.targets(sourceId))) {
        if (id !== sourceId)
          ports.send(
            id,
            new TextEncoder().encode(
              cursorInput(packet.data, ports.applicationCursor(id)),
            ),
          );
      }
    },
    wheel(sourceId: string, event: WheelEvent) {
      if (localDepth.has(sourceId)) return;
      for (const id of new Set(ports.targets(sourceId))) {
        if (id !== sourceId) local(id, () => ports.wheel(id, event, sourceId));
      }
    },
  };
}

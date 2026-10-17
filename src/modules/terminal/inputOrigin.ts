import type { IDisposable, Terminal } from "@xterm/xterm";
import type { FinalWheelInput } from "./inputRouter";

export interface TerminalInput {
  data: string;
  binary: boolean;
  userInput: boolean;
  wheel: boolean;
  wheelEvent?: WheelEvent;
}

interface MouseReportPosition {
  col: number;
  row: number;
  x: number;
  y: number;
}

/** 固定版本的内部能力集中在此处；不替换库方法，不触碰滚轮增量累积器。 */
function inputCore(term: Terminal) {
  return (
    term as unknown as {
      _core?: {
        coreService?: { onUserInput?: (listener: () => void) => IDisposable };
        coreMouseService?: {
          triggerMouseEvent?: (
            event: MouseReportPosition & {
              button: number;
              action: number;
              ctrl: boolean;
              alt: boolean;
              shift: boolean;
            },
          ) => boolean;
        };
        _mouseService?: {
          getMouseReportCoords?: (
            event: { clientX: number; clientY: number },
            screen: HTMLElement,
          ) => MouseReportPosition | undefined;
        };
      };
    }
  )._core;
}

/** 鼠标最终动作按目标协议/坐标编码一次；方向键只适配目标应用光标模式。 */
export function sendFinalWheelInput(
  term: Terminal,
  input: FinalWheelInput,
  source: Terminal,
  event?: WheelEvent,
) {
  const mouse = term.modes.mouseTrackingMode;
  const mouseWheel = mouse !== "none" && mouse !== "x10";
  if (!mouseWheel && term.buffer.active.type !== "alternate") return false;
  if (!input.mouse || !mouseWheel) {
    const prefix = term.modes.applicationCursorKeysMode ? "O" : "[";
    term.input(`\x1b${prefix}${input.direction === "up" ? "A" : "B"}`, true);
    return true;
  }
  const from = source.element?.querySelector<HTMLElement>(".xterm-screen");
  const to = term.element?.querySelector<HTMLElement>(".xterm-screen");
  const core = inputCore(term);
  if (
    !event ||
    !from ||
    !to ||
    !core?._mouseService?.getMouseReportCoords ||
    !core.coreMouseService?.triggerMouseEvent
  )
    return false;
  const fromRect = from.getBoundingClientRect();
  const toRect = to.getBoundingClientRect();
  if (
    fromRect.width <= 0 ||
    fromRect.height <= 0 ||
    toRect.width <= 0 ||
    toRect.height <= 0
  )
    return false;
  const ratio = (value: number) => Math.max(0, Math.min(1, value));
  const pos = core._mouseService.getMouseReportCoords(
    {
      clientX:
        toRect.left +
        ratio((event.clientX - fromRect.left) / fromRect.width) * toRect.width,
      clientY:
        toRect.top +
        ratio((event.clientY - fromRect.top) / fromRect.height) * toRect.height,
    },
    to,
  );
  if (!pos) return false;
  // xterm 6 的 WHEEL=4、UP=0、DOWN=1；编码器自行处理 SGR/像素/二进制及边界。
  return core.coreMouseService.triggerMouseEvent({
    ...pos,
    button: 4,
    action: input.direction === "up" ? 0 : 1,
    ctrl: input.ctrl,
    alt: input.alt,
    shift: input.shift,
  });
}

/**
 * xterm 6.0.0 的公开onData混合用户输入与设备应答，公开onKey又不覆盖IME。
 * 用户标记在紧随其后的onData中消费；滚轮标记从自定义编码回调进入，避免捕获阶段提前清理。
 * 广播依赖的内部能力缺失时仍单终端发送，但禁止开启广播；升级xterm须复核事件顺序。
 */
export function observeTerminalInput(
  term: Terminal,
  receive: (input: TerminalInput) => void,
) {
  const core = inputCore(term);
  let userInput = false;
  let wheelEvent: WheelEvent | undefined;
  let wheelRevision = 0;
  const marker = core?.coreService?.onUserInput?.(() => {
    userInput = true;
  });
  const data = term.onData((value) => {
    const fromUser = userInput;
    userInput = false;
    receive({
      data: value,
      binary: false,
      userInput: fromUser,
      wheel: !!wheelEvent,
      wheelEvent,
    });
  });
  const binary = term.onBinary((value) => {
    userInput = false;
    receive({
      data: value,
      binary: true,
      userInput: false,
      wheel: !!wheelEvent,
      wheelEvent,
    });
  });
  return {
    supported:
      !!marker &&
      !!core?.coreMouseService?.triggerMouseEvent &&
      !!core?._mouseService?.getMouseReportCoords,
    markWheelInput(event: WheelEvent) {
      // 从 xterm 自定义回调进入：编码与 onData/onBinary 在同一 JS 调用栈内完成。
      wheelEvent = event;
      const revision = ++wheelRevision;
      queueMicrotask(() => {
        if (revision !== wheelRevision) return;
        wheelEvent = undefined;
      });
    },
    dispose() {
      wheelRevision++;
      wheelEvent = undefined;
      marker?.dispose();
      data.dispose();
      binary.dispose();
    },
  };
}

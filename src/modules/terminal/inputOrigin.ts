import type { IDisposable, Terminal } from "@xterm/xterm";
import type { FinalWheelInput } from "./inputRouter";
import { convertChinesePunctuation } from "./punctuation";

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
        readonly _keyPressHandled?: boolean;
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
  punctuationEnabled: () => boolean,
) {
  const core = inputCore(term);
  const element = term.element;
  let composing = false;
  let compositionTimer: ReturnType<typeof setTimeout> | undefined;
  const compositionStart = (event: CompositionEvent) => {
    if (event.target !== term.textarea) return;
    if (compositionTimer !== undefined) clearTimeout(compositionTimer);
    compositionTimer = undefined;
    composing = true;
  };
  const compositionEnd = (event: CompositionEvent) => {
    if (event.target !== term.textarea) return;
    // 在冒泡阶段等待 xterm 已安排的组合提交，避免直接输入抢先或重复发送组合正文。
    if (compositionTimer !== undefined) clearTimeout(compositionTimer);
    compositionTimer = setTimeout(() => {
      composing = false;
      compositionTimer = undefined;
    }, 0);
  };
  const punctuationInput = (event: InputEvent) => {
    if (
      event.target !== term.textarea ||
      event.defaultPrevented ||
      !event.cancelable ||
      event.inputType !== "insertText" ||
      event.isComposing ||
      composing ||
      core?._keyPressHandled !== false ||
      !event.data ||
      !punctuationEnabled()
    )
      return;
    const converted = convertChinesePunctuation(event.data);
    if (converted === event.data) return;
    // 沿用 xterm 的 keypress 去重状态；能力缺失时不接管，不重复发送已处理的字符。
    // WebKit 可先提交 Shift 标点再发 keydown；只接管已确认文本，不按物理键猜字符。
    // 取消原生插入后仅发送一次，迟到的 keydown 也不会从 textarea 差量重复提取标点。
    event.preventDefault();
    event.stopPropagation();
    term.input(converted, true);
  };
  const ignoreModifierKeydown = (event: KeyboardEvent) => {
    if (
      event.target === term.textarea &&
      ["Meta", "Control", "Alt", "Shift"].includes(event.key)
    ) {
      // xterm 6 在自定义按键回调前标记 keydown；WebKit 的 Shift 标点可能先发 input。
      // 提前过滤纯修饰键，保留原生输入法行为及前一字符的去重状态，避免首个标点丢失。
      event.stopPropagation();
    }
  };
  element?.addEventListener("compositionstart", compositionStart, true);
  element?.addEventListener("compositionend", compositionEnd);
  element?.addEventListener("beforeinput", punctuationInput, true);
  element?.addEventListener("keydown", ignoreModifierKeydown, true);
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
      if (compositionTimer !== undefined) clearTimeout(compositionTimer);
      element?.removeEventListener("compositionstart", compositionStart, true);
      element?.removeEventListener("compositionend", compositionEnd);
      element?.removeEventListener("beforeinput", punctuationInput, true);
      element?.removeEventListener("keydown", ignoreModifierKeydown, true);
      marker?.dispose();
      data.dispose();
      binary.dispose();
    },
  };
}

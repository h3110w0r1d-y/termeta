import type { IDisposable, Terminal } from "@xterm/xterm";

export interface TerminalInput {
  data: string;
  binary: boolean;
  userInput: boolean;
  wheel: boolean;
}

/**
 * xterm 6.0.0 的公开onData混合用户输入与设备应答，公开onKey又不覆盖IME。
 * 只订阅内部同步onUserInput标记，不替换方法或修改库；升级xterm须复核事件顺序。
 * 标记在紧随其后的onData中消费，缺少此能力时仍单终端发送，但禁止开启广播。
 */
export function observeTerminalInput(
  term: Terminal,
  receive: (input: TerminalInput) => void,
) {
  const core = (
    term as unknown as {
      _core?: {
        coreService?: { onUserInput?: (listener: () => void) => IDisposable };
      };
    }
  )._core?.coreService;
  let userInput = false;
  let wheel = false;
  const marker = core?.onUserInput?.(() => {
    userInput = true;
  });
  const data = term.onData((value) => {
    const fromUser = userInput;
    userInput = false;
    receive({ data: value, binary: false, userInput: fromUser, wheel });
  });
  const binary = term.onBinary((value) => {
    userInput = false;
    receive({ data: value, binary: true, userInput: false, wheel });
  });
  // 滚轮也可能编码成方向键；限定在当前DOM事件轮次，不能作为键盘再次广播。
  const onWheel = () => {
    wheel = true;
    queueMicrotask(() => {
      wheel = false;
    });
  };
  const element = term.element;
  element?.addEventListener("wheel", onWheel, true);
  return {
    supported: !!marker,
    dispose() {
      marker?.dispose();
      data.dispose();
      binary.dispose();
      element?.removeEventListener("wheel", onWheel, true);
    },
  };
}

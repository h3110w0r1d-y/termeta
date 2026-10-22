import type { TerminalMouseAction } from "../settings/model";

interface MouseActionOptions {
  isMac: boolean;
  action: (button: 1 | 2) => TerminalMouseAction;
  canPaste: () => boolean;
  focus: () => void;
  paste: () => Promise<void>;
  onError: (error: unknown) => void;
}

/** 本地粘贴手势在 xterm 编码前消费；无动作仍保留远端鼠标协议。 */
export function bindTerminalMouseActions(
  host: HTMLElement,
  options: MouseActionOptions,
) {
  function button(event: MouseEvent) {
    if (event.button === 1) return 1;
    if (
      event.button === 2 ||
      (options.isMac && event.button === 0 && event.ctrlKey)
    )
      return 2;
    return undefined;
  }
  function consume(event: MouseEvent) {
    event.preventDefault();
    event.stopImmediatePropagation();
  }
  function down(event: MouseEvent) {
    const key = button(event);
    if (!key) return;
    // 阻止系统中键自动滚动和 PRIMARY 选区粘贴，剪贴板只从统一入口读取。
    if (key === 1) event.preventDefault();
    if (options.action(key) !== "paste") return;
    consume(event);
    if (!options.canPaste()) return;
    options.focus();
    // 在真实按下事件内读取剪贴板，避免等待后丢失浏览器的用户激活上下文。
    void options.paste().catch(options.onError);
  }
  function up(event: MouseEvent) {
    const key = button(event);
    if (key && options.action(key) === "paste") consume(event);
  }
  function context(event: MouseEvent) {
    // 禁用原生菜单及 xterm 的原生粘贴准备，避免一个手势重复粘贴。
    consume(event);
  }
  function auxiliary(event: MouseEvent) {
    if (button(event)) consume(event);
  }
  host.addEventListener("mousedown", down, true);
  host.addEventListener("mouseup", up, true);
  host.addEventListener("contextmenu", context, true);
  host.addEventListener("auxclick", auxiliary, true);
  return () => {
    host.removeEventListener("mousedown", down, true);
    host.removeEventListener("mouseup", up, true);
    host.removeEventListener("contextmenu", context, true);
    host.removeEventListener("auxclick", auxiliary, true);
  };
}

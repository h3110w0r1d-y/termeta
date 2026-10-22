import { onBeforeUnmount } from "vue";
import { desktop, errorMessage } from "@/services/desktop/client";

interface WindowDragOptions {
  enabled: () => boolean;
  onError: (message: string) => void;
  threshold?: number;
}

interface PendingDrag {
  pointerId: number;
  x: number;
  y: number;
}

/** 可点击 Header 控件的窗口拖动手势；移动超过阈值后才交给原生窗口。 */
export function useWindowDrag(options: WindowDragOptions) {
  const threshold = options.threshold ?? 6;
  let pending: PendingDrag | undefined;
  let moved = false;
  let suppressClick = false;
  let suppressTimer: ReturnType<typeof setTimeout> | undefined;

  function start(event: PointerEvent) {
    if (
      !options.enabled() ||
      event.button !== 0 ||
      !event.isPrimary ||
      event.pointerType === "touch" ||
      pending
    )
      return;
    clearTimeout(suppressTimer);
    suppressTimer = undefined;
    suppressClick = false;
    window.removeEventListener("click", swallowClick, true);
    pending = {
      pointerId: event.pointerId,
      x: event.clientX,
      y: event.clientY,
    };
    moved = false;
    window.addEventListener("pointermove", move, {
      capture: true,
      passive: false,
    });
    window.addEventListener("pointerup", finish, true);
    window.addEventListener("pointercancel", finish, true);
    window.addEventListener("blur", cancel);
  }

  function move(event: PointerEvent) {
    if (!pending || event.pointerId !== pending.pointerId) return;
    if (!options.enabled()) {
      cancel();
      return;
    }
    if (
      !moved &&
      Math.hypot(event.clientX - pending.x, event.clientY - pending.y) <
        threshold
    )
      return;
    if (!moved) {
      moved = true;
      suppressClick = true;
      clearTimeout(suppressTimer);
      window.addEventListener("click", swallowClick, true);
      event.preventDefault();
      void desktop.startWindowDragging().catch((cause) => {
        options.onError(errorMessage(cause));
      });
    }
    event.preventDefault();
  }

  function finish(event: PointerEvent) {
    if (!pending || event.pointerId !== pending.pointerId) return;
    cleanup();
  }

  function cancel() {
    cleanup();
  }

  function cleanup() {
    pending = undefined;
    moved = false;
    window.removeEventListener("pointermove", move, true);
    window.removeEventListener("pointerup", finish, true);
    window.removeEventListener("pointercancel", finish, true);
    window.removeEventListener("blur", cancel);
    if (!suppressClick) return;
    clearTimeout(suppressTimer);
    suppressTimer = setTimeout(() => {
      suppressClick = false;
      window.removeEventListener("click", swallowClick, true);
    }, 0);
  }

  function swallowClick(event: MouseEvent) {
    if (!suppressClick) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  }

  onBeforeUnmount(() => {
    cancel();
    clearTimeout(suppressTimer);
    suppressTimer = undefined;
    suppressClick = false;
    window.removeEventListener("click", swallowClick, true);
  });

  return { start };
}

import { computed, onScopeDispose, shallowRef, type CSSProperties } from "vue";
import {
  createSftpFileDrag,
  type DragPoint,
  type SftpDragFeedback,
  type SftpDragSelection,
} from "./sftpFileDrag";

export function useSftpFileDrag(options: {
  valid: (selection: SftpDragSelection) => boolean;
  terminalHost: () => HTMLElement | undefined;
  paste: (selection: SftpDragSelection) => void;
  native: (selection: SftpDragSelection) => void;
}) {
  const feedback = shallowRef<SftpDragFeedback>();
  let capture: { element: HTMLElement; id: number } | undefined;
  function release() {
    const current = capture;
    capture = undefined;
    window.removeEventListener("pointermove", move, true);
    window.removeEventListener("pointerup", end, true);
    window.removeEventListener("pointercancel", cancelPointer, true);
    window.removeEventListener("lostpointercapture", cancelPointer, true);
    window.removeEventListener("blur", drag.cancel);
    window.removeEventListener("keydown", keydown, true);
    if (current?.element.hasPointerCapture(current.id))
      current.element.releasePointerCapture(current.id);
  }
  function overTerminal(point: DragPoint) {
    const host = options.terminalHost();
    const hit = document.elementFromPoint(point.x, point.y);
    return !!host && !!hit && host.contains(hit);
  }
  const drag = createSftpFileDrag({
    ...options,
    valid: (selection) =>
      options.valid(selection) &&
      !document.querySelector("[data-modal-open], [data-ui-popup-open]"),
    insideWindow: ({ x, y }) =>
      x >= 0 && y >= 0 && x < window.innerWidth && y < window.innerHeight,
    overTerminal,
    update(value) {
      feedback.value = value;
      if (!value) release();
    },
  });
  function move(event: PointerEvent) {
    if (event.pointerId !== capture?.id) return;
    drag.move(
      event.pointerId,
      { x: event.clientX, y: event.clientY },
      event.buttons,
    );
    if (feedback.value?.active) event.preventDefault();
  }
  function end(event: PointerEvent) {
    if (event.pointerId !== capture?.id) return;
    if (feedback.value?.active) {
      event.preventDefault();
      event.stopPropagation();
    }
    drag.end(event.pointerId, { x: event.clientX, y: event.clientY });
  }
  function cancelPointer(event: PointerEvent) {
    if (event.pointerId === capture?.id) drag.cancel();
  }
  function keydown(event: KeyboardEvent) {
    if (event.key !== "Escape") return;
    event.preventDefault();
    event.stopPropagation();
    drag.cancel();
  }
  function start(event: PointerEvent, selection: SftpDragSelection) {
    if (!event.isPrimary || event.button !== 0) return;
    if (
      !drag.start(
        event.pointerId,
        { x: event.clientX, y: event.clientY },
        selection,
      )
    )
      return;
    const element = event.currentTarget as HTMLElement;
    capture = { element, id: event.pointerId };
    try {
      element.setPointerCapture(event.pointerId);
    } catch {
      drag.cancel();
      return;
    }
    event.preventDefault();
    window.addEventListener("pointermove", move, {
      capture: true,
      passive: false,
    });
    window.addEventListener("pointerup", end, true);
    window.addEventListener("pointercancel", cancelPointer, true);
    window.addEventListener("lostpointercapture", cancelPointer, true);
    window.addEventListener("blur", drag.cancel);
    window.addEventListener("keydown", keydown, true);
  }
  onScopeDispose(drag.cancel);
  const previewStyle = computed<CSSProperties>(() =>
    feedback.value
      ? {
          left:
            Math.max(
              8,
              Math.min(feedback.value.x + 14, window.innerWidth - 290),
            ) + "px",
          top:
            Math.max(
              8,
              Math.min(feedback.value.y + 16, window.innerHeight - 70),
            ) + "px",
        }
      : {},
  );
  return {
    start,
    cancel: drag.cancel,
    feedback,
    previewStyle,
    dragging: computed(() => !!feedback.value),
  };
}

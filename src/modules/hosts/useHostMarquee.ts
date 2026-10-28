import {
  computed,
  onBeforeUnmount,
  ref,
  watch,
  type CSSProperties,
  type Ref,
} from "vue";
import { isMac } from "@/services/commands/registry";

interface HostMarqueeOptions {
  root: Ref<HTMLElement | undefined>;
  enabled: () => boolean;
  visibleIds: () => string[];
  selectedIds: Ref<string[]>;
  onSelection: (ids: string[]) => void;
  onFocus: () => void;
}

interface MarqueeGesture {
  id: number;
  root: HTMLElement;
  viewport: HTMLElement;
  startX: number;
  startY: number;
  selectionX: number;
  selectionY: number;
  scrollLeft: number;
  scrollTop: number;
  clientX: number;
  clientY: number;
  initial: string[];
  additive: boolean;
}

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, value));

// 空白框选只更新选区；卡片自身的连接、拖动排序与原生选字仍由各自入口处理。
export function useHostMarquee(options: HostMarqueeOptions) {
  const active = ref(false);
  const bounds = ref<{
    left: number;
    top: number;
    width: number;
    height: number;
  }>();
  const rectangle = computed<CSSProperties | undefined>(() =>
    bounds.value
      ? {
          left: `${bounds.value.left}px`,
          top: `${bounds.value.top}px`,
          width: `${bounds.value.width}px`,
          height: `${bounds.value.height}px`,
        }
      : undefined,
  );
  let gesture: MarqueeGesture | undefined;
  let frame = 0;
  let blockClick = false;

  function publish(ids: string[]) {
    if (
      ids.length !== options.selectedIds.value.length ||
      ids.some((id, index) => id !== options.selectedIds.value[index])
    )
      options.onSelection(ids);
  }

  function update() {
    const current = gesture;
    if (!current || !active.value) return;
    const rootBox = current.root.getBoundingClientRect();
    const left = rootBox.left + current.root.clientLeft;
    const top = rootBox.top + current.root.clientTop;
    const right = left + current.root.clientWidth;
    const bottom = top + current.root.clientHeight;
    const dx = current.viewport.scrollLeft - current.scrollLeft;
    const dy = current.viewport.scrollTop - current.scrollTop;
    const startX = current.startX - dx;
    const startY = current.startY - dy;
    const endX = clamp(current.clientX, left, right);
    const endY = clamp(current.clientY, top, bottom);
    const rectangleLeft = clamp(Math.min(startX, endX), left, right);
    const rectangleTop = clamp(Math.min(startY, endY), top, bottom);
    bounds.value = {
      left: rectangleLeft - left,
      top: rectangleTop - top,
      width: Math.max(
        0,
        Math.min(Math.max(startX, endX), right) - rectangleLeft,
      ),
      height: Math.max(
        0,
        Math.min(Math.max(startY, endY), bottom) - rectangleTop,
      ),
    };

    // 外侧留白可起拖，但碰撞端点限制在真实视口，避免选中尚未滚入的条目。
    const viewportBox = current.viewport.getBoundingClientRect();
    const selectionEndX = clamp(
      current.clientX,
      viewportBox.left,
      viewportBox.right,
    );
    const selectionEndY = clamp(
      current.clientY,
      viewportBox.top,
      viewportBox.bottom,
    );
    const selectionLeft = Math.min(current.selectionX - dx, selectionEndX);
    const selectionTop = Math.min(current.selectionY - dy, selectionEndY);
    const selectionRight = Math.max(current.selectionX - dx, selectionEndX);
    const selectionBottom = Math.max(current.selectionY - dy, selectionEndY);
    const selected = new Set(current.additive ? current.initial : []);
    if (selectionRight > selectionLeft && selectionBottom > selectionTop) {
      for (const item of current.root.querySelectorAll<HTMLElement>(
        "[data-host-order-id]",
      )) {
        const id = item.dataset.hostOrderId;
        const box = item.getBoundingClientRect();
        if (
          id &&
          box.width > 0 &&
          box.height > 0 &&
          box.left < selectionRight &&
          box.right > selectionLeft &&
          box.top < selectionBottom &&
          box.bottom > selectionTop
        )
          selected.add(id);
      }
    }
    publish(options.visibleIds().filter((id) => selected.has(id)));
  }

  function scrollFrame() {
    frame = 0;
    const current = gesture;
    if (!current || !active.value) return;
    if (!options.enabled()) {
      cancel();
      return;
    }
    const rootBox = current.root.getBoundingClientRect();
    const box = current.viewport.getBoundingClientRect();
    const speed = (position: number, minimum: number, maximum: number) =>
      position < minimum + 32
        ? -Math.min(18, (minimum + 32 - position) / 2)
        : position > maximum - 32
          ? Math.min(18, (position - maximum + 32) / 2)
          : 0;
    // 左右留白内的指针也可触发纵向滚动，不要求落在滚动元素上。
    if (current.clientX >= rootBox.left && current.clientX <= rootBox.right)
      current.viewport.scrollTop += speed(current.clientY, box.top, box.bottom);
    if (current.clientY >= rootBox.top && current.clientY <= rootBox.bottom)
      current.viewport.scrollLeft += speed(
        current.clientX,
        box.left,
        box.right,
      );
    update();
    frame = requestAnimationFrame(scrollFrame);
  }

  function finish(restore: boolean) {
    const current = gesture;
    gesture = undefined;
    active.value = false;
    bounds.value = undefined;
    cancelAnimationFrame(frame);
    frame = 0;
    window.removeEventListener("pointermove", move, true);
    window.removeEventListener("pointerup", end, true);
    window.removeEventListener("pointercancel", cancelPointer, true);
    window.removeEventListener("keydown", cancelKey, true);
    window.removeEventListener("blur", cancel);
    if (!current) return;
    current.root.removeEventListener("lostpointercapture", lostCapture);
    if (current.root.hasPointerCapture(current.id))
      current.root.releasePointerCapture(current.id);
    if (restore) {
      const initial = new Set(current.initial);
      publish(options.visibleIds().filter((id) => initial.has(id)));
    }
  }

  function cancel() {
    finish(true);
  }

  function move(event: PointerEvent) {
    const current = gesture;
    if (!current || current.id !== event.pointerId) return;
    if (!options.enabled() || !(event.buttons & 1)) {
      cancel();
      return;
    }
    current.clientX = event.clientX;
    current.clientY = event.clientY;
    if (
      !active.value &&
      Math.hypot(
        current.clientX - current.startX,
        current.clientY - current.startY,
      ) >= 5
    ) {
      active.value = true;
      blockClick = true;
      window.getSelection()?.removeAllRanges();
      // 越过阈值后才捕获，普通空白点击不进入框选态。
      try {
        current.root.setPointerCapture(current.id);
      } catch {
        cancel();
        return;
      }
      frame = requestAnimationFrame(scrollFrame);
    }
    if (active.value) {
      event.preventDefault();
      event.stopPropagation();
      update();
    }
  }

  function end(event: PointerEvent) {
    const current = gesture;
    if (!current || current.id !== event.pointerId) return;
    if (!options.enabled()) {
      cancel();
      return;
    }
    current.clientX = event.clientX;
    current.clientY = event.clientY;
    if (active.value) update();
    else if (!current.additive) publish([]);
    blockClick = true;
    event.preventDefault();
    event.stopPropagation();
    finish(false);
  }

  function cancelPointer(event: PointerEvent) {
    if (gesture?.id === event.pointerId) cancel();
  }

  function lostCapture(event: PointerEvent) {
    // 子元素交出隐式捕获时也会冒泡，只处理框选根自身丢失的捕获。
    if (gesture?.id === event.pointerId && event.target === gesture.root)
      cancel();
  }

  function cancelKey(event: KeyboardEvent) {
    if (event.key !== "Escape" || !gesture) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    cancel();
  }

  function suppressClick(event: MouseEvent) {
    // 键盘或辅助技术生成的点击不属于鼠标松手后的兼容事件。
    if (!blockClick || event.detail === 0) return;
    blockClick = false;
    event.preventDefault();
    event.stopImmediatePropagation();
  }

  function start(event: PointerEvent) {
    if (!gesture && event.isPrimary && event.button === 0) blockClick = false;
    const root = options.root.value;
    if (
      gesture ||
      !options.enabled() ||
      !root ||
      !event.isPrimary ||
      event.button !== 0 ||
      (isMac && event.ctrlKey) ||
      event.pointerType === "touch" ||
      !(event.target instanceof Element) ||
      !root.contains(event.target) ||
      event.target.closest(
        "[data-host-order-id], .selectable-text, button, input, textarea, select, a, [contenteditable], [role=button], [role=scrollbar], .n-scrollbar-rail, .n-checkbox",
      )
    )
      return;
    const viewport =
      root.querySelector<HTMLElement>(".n-scrollbar-container") ?? root;
    const box = viewport.getBoundingClientRect();
    gesture = {
      id: event.pointerId,
      root,
      viewport,
      startX: event.clientX,
      startY: event.clientY,
      selectionX: clamp(event.clientX, box.left, box.right),
      selectionY: clamp(event.clientY, box.top, box.bottom),
      scrollLeft: viewport.scrollLeft,
      scrollTop: viewport.scrollTop,
      clientX: event.clientX,
      clientY: event.clientY,
      initial: [...options.selectedIds.value],
      additive: event.ctrlKey || event.metaKey || event.shiftKey,
    };
    event.preventDefault();
    options.onFocus();
    root.addEventListener("lostpointercapture", lostCapture);
    window.addEventListener("pointermove", move, {
      capture: true,
      passive: false,
    });
    window.addEventListener("pointerup", end, true);
    window.addEventListener("pointercancel", cancelPointer, true);
    window.addEventListener("keydown", cancelKey, true);
    window.addEventListener("blur", cancel);
  }

  watch(options.enabled, (enabled) => {
    if (!enabled) cancel();
  });
  watch(options.visibleIds, (ids, previous) => {
    if (
      ids.length !== previous.length ||
      ids.some((id, index) => id !== previous[index])
    )
      cancel();
  });
  watch(options.root, cancel);
  onBeforeUnmount(cancel);
  return { rectangle, active, start, cancel, suppressClick };
}

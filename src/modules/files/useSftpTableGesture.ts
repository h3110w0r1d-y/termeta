import { onBeforeUnmount, ref, watch, type Ref } from "vue";
import type { FileEntry } from "@/services/desktop/types";

export function isSftpTableControl(target: EventTarget | null) {
  return (
    target instanceof Element &&
    !!target.closest(".n-checkbox, button, input, a")
  );
}

interface BoxGesture {
  id: number;
  viewport: HTMLElement;
  startX: number;
  startY: number;
  x: number;
  y: number;
  clientX: number;
  clientY: number;
  contentOffset: number;
  initial: string[];
  additive: boolean;
  active: boolean;
  first?: number;
  last?: number;
}
interface TableGestureOptions {
  root: Ref<HTMLElement | undefined>;
  rows: Readonly<Ref<FileEntry[]>>;
  selected: Ref<string[]>;
  unavailable: Readonly<Ref<boolean>>;
  rowHeight: number;
  completeBoxSelection: () => void;
}

// 只适配表格指针手势和虚拟滚动，不负责行选择规则或文件传输。
export function useSftpTableGesture(options: TableGestureOptions) {
  const { root, rows, selected, unavailable, rowHeight } = options;
  const suppressClick = ref(false);
  let nameGesture: { id: number; x: number; y: number } | undefined;

  function startName(event: PointerEvent, track: boolean) {
    suppressClick.value = false;
    if (!track) return;
    // 名称拖动的捕获与阈值由调用方管理，这里只阻止松手后的误点击。
    nameGesture = { id: event.pointerId, x: event.clientX, y: event.clientY };
    watchPointer();
  }
  function focusTable() {
    root.value?.focus({ preventScroll: true });
  }
  let box: BoxGesture | undefined;
  const rectangle = ref<{
    left: number;
    top: number;
    width: number;
    height: number;
  }>();
  let frame = 0;

  function startBox(event: PointerEvent) {
    if (
      !event.isPrimary ||
      event.button !== 0 ||
      unavailable.value ||
      !(event.target instanceof Element)
    )
      return;
    suppressClick.value = false;
    if (
      isSftpTableControl(event.target) ||
      event.target.closest(".sftp-name-cell, .n-data-table-thead")
    )
      return;
    // 将 Naive 虚拟滚动 DOM 依赖集中在此适配边界，命中通过完整数据索引计算。
    const viewport = root.value?.querySelector<HTMLElement>(".v-vl");
    if (!viewport) return;
    const bounds = viewport.getBoundingClientRect();
    if (
      event.clientX < bounds.left ||
      event.clientX >= bounds.left + viewport.clientWidth ||
      event.clientY < bounds.top ||
      event.clientY >= bounds.top + viewport.clientHeight
    )
      return;
    const sample = viewport.querySelector<HTMLElement>("[data-sftp-row-index]");
    const offset = sample
      ? sample.getBoundingClientRect().top -
        bounds.top +
        viewport.scrollTop -
        Number(sample.dataset.sftpRowIndex) * rowHeight
      : 0;
    event.preventDefault();
    focusTable();
    box = {
      id: event.pointerId,
      viewport,
      startX: event.clientX,
      startY: event.clientY - bounds.top + viewport.scrollTop,
      x: event.clientX,
      y: event.clientY - bounds.top + viewport.scrollTop,
      clientX: event.clientX,
      clientY: event.clientY,
      contentOffset: offset,
      initial: [...selected.value],
      additive: event.ctrlKey || event.metaKey || event.shiftKey,
      active: false,
    };
    watchPointer();
  }
  function updateBox() {
    if (!box?.active || !root.value) return;
    const bounds = box.viewport.getBoundingClientRect();
    box.x = Math.max(bounds.left, Math.min(bounds.right, box.clientX));
    box.y = Math.max(
      0,
      Math.min(
        box.viewport.scrollHeight,
        box.clientY - bounds.top + box.viewport.scrollTop,
      ),
    );
    const top = Math.min(box.startY, box.y) - box.contentOffset;
    const bottom = Math.max(box.startY, box.y) - box.contentOffset;
    const first = Math.max(1, Math.floor(top / rowHeight));
    const last = Math.min(
      rows.value.length - 1,
      Math.ceil(bottom / rowHeight) - 1,
    );
    if (first !== box.first || last !== box.last) {
      const names = rows.value
        .slice(first, Math.max(first, last + 1))
        .map((row) => row.name);
      selected.value = box.additive
        ? [...new Set([...box.initial, ...names])]
        : names;
      // 只在跨越行边界时更新选区，避免自动滚动帧重复驱动上层大列表计算。
      box.first = first;
      box.last = last;
    }
    const rootBounds = root.value.getBoundingClientRect();
    const visibleTop = Math.max(
      bounds.top,
      Math.min(box.startY, box.y) + bounds.top - box.viewport.scrollTop,
    );
    const visibleBottom = Math.min(
      bounds.bottom,
      Math.max(box.startY, box.y) + bounds.top - box.viewport.scrollTop,
    );
    rectangle.value = {
      left: Math.min(box.startX, box.x) - rootBounds.left,
      top: visibleTop - rootBounds.top,
      width: Math.abs(box.x - box.startX),
      height: Math.max(0, visibleBottom - visibleTop),
    };
  }
  function autoScroll() {
    frame = 0;
    if (!box?.active) return;
    const bounds = box.viewport.getBoundingClientRect();
    const edge = 32;
    const speed =
      box.clientY < bounds.top + edge
        ? -Math.min(18, (bounds.top + edge - box.clientY) / 2)
        : box.clientY > bounds.bottom - edge
          ? Math.min(18, (box.clientY - bounds.bottom + edge) / 2)
          : 0;
    if (speed) box.viewport.scrollTop += speed;
    updateBox();
    frame = requestAnimationFrame(autoScroll);
  }
  function movePointer(event: PointerEvent) {
    if (unavailable.value) {
      cancelGesture(true);
      return;
    }
    if (
      nameGesture?.id === event.pointerId &&
      Math.hypot(
        event.clientX - nameGesture.x,
        event.clientY - nameGesture.y,
      ) >= 6
    )
      suppressClick.value = true;
    if (!box || box.id !== event.pointerId) return;
    if (!(event.buttons & 1)) {
      cancelGesture(true);
      return;
    }
    box.clientX = event.clientX;
    box.clientY = event.clientY;
    if (
      !box.active &&
      Math.hypot(
        event.clientX - box.startX,
        event.clientY -
          (box.startY +
            box.viewport.getBoundingClientRect().top -
            box.viewport.scrollTop),
      ) >= 6
    ) {
      box.active = true;
      suppressClick.value = true;
      // 等越过阈值才捕获；普通点击仍由原始行接收，保持点击与框选互斥。
      root.value?.setPointerCapture(event.pointerId);
      frame = requestAnimationFrame(autoScroll);
    }
    if (box.active) {
      event.preventDefault();
      updateBox();
    }
  }
  function endPointer(event: PointerEvent) {
    if (
      (box && box.id !== event.pointerId) ||
      (nameGesture && nameGesture.id !== event.pointerId)
    )
      return;
    if (box?.active) {
      updateBox();
      options.completeBoxSelection();
    }
    cancelGesture(false);
  }
  function cancelPointer(event: PointerEvent) {
    if (box?.id === event.pointerId || nameGesture?.id === event.pointerId)
      cancelGesture(true);
  }
  function lostCapture(event: PointerEvent) {
    if (box?.id === event.pointerId) cancelGesture(true);
  }
  function cancelGesture(restore: boolean) {
    const current = box;
    box = undefined;
    nameGesture = undefined;
    rectangle.value = undefined;
    cancelAnimationFrame(frame);
    frame = 0;
    if (restore && current?.active) selected.value = current.initial;
    if (current && root.value?.hasPointerCapture(current.id))
      root.value.releasePointerCapture(current.id);
    window.removeEventListener("pointermove", movePointer, true);
    window.removeEventListener("pointerup", endPointer, true);
    window.removeEventListener("pointercancel", cancelPointer, true);
    window.removeEventListener("blur", blurGesture);
  }
  function blurGesture() {
    cancelGesture(true);
  }
  function watchPointer() {
    window.addEventListener("pointermove", movePointer, {
      capture: true,
      passive: false,
    });
    window.addEventListener("pointerup", endPointer, true);
    window.addEventListener("pointercancel", cancelPointer, true);
    window.addEventListener("blur", blurGesture);
  }
  function captureClick(event: MouseEvent) {
    if (suppressClick.value) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    if (unavailable.value || !(event.target instanceof Element)) return;
    if (
      event.target.closest(".v-vl") &&
      !event.target.closest("[data-sftp-row-index]") &&
      !event.ctrlKey &&
      !event.metaKey &&
      !event.shiftKey
    )
      selected.value = [];
  }

  function cancelKey(event: KeyboardEvent) {
    if (event.key !== "Escape" || (!box && !nameGesture)) return false;
    event.preventDefault();
    cancelGesture(true);
    return true;
  }
  watch(unavailable, (value) => {
    if (value) cancelGesture(false);
  });
  onBeforeUnmount(() => cancelGesture(false));

  return {
    suppressClick,
    rectangle,
    startName,
    startBox,
    lostCapture,
    captureClick,
    cancelKey,
    cancelGesture,
  };
}

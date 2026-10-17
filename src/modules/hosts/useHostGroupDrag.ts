import { t } from "../../i18n";
import { onBeforeUnmount, ref, type Ref } from "vue";
import type { HostProfile } from "../../services/desktop/types";
import { errorMessage } from "../../services/desktop/client";
import { descendantIds } from "./groups";
import { useHosts } from "./store";
import { createDragPreview } from "./dragPreview";

export function useHostGroupDrag(options: {
  panel: Ref<HTMLElement | undefined>;
  previewLayer: Ref<HTMLElement | undefined>;
  reveal: (id: string | null) => void;
  nextOrder: (id: string | null) => number;
  closeMenu: () => void;
  onError: (message: string) => void;
}) {
  const store = useHosts();
  const { panel, previewLayer, reveal, nextOrder } = options;
  interface DragState {
    kind: "group" | "host";
    id: string;
    label: string;
    pointer: number;
    x: number;
    y: number;
    moved: boolean;
    target: string | null;
  }
  const drag = ref<DragState>();
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;
  let capture: HTMLElement | undefined;
  let preview: ReturnType<typeof createDragPreview> | undefined;
  let suppressClick = false;
  let suppressTimer: ReturnType<typeof setTimeout> | undefined;
  function startDrag(
    event: PointerEvent,
    kind: DragState["kind"],
    id: string,
    label: string,
  ) {
    if (
      event.button !== 0 ||
      event.pointerType === "touch" ||
      store.managementBusy ||
      drag.value ||
      (event.target as HTMLElement).closest(
        "button, input, .n-tree-node-switcher, .selectable-text, [contenteditable]",
      )
    )
      return;
    drag.value = {
      kind,
      id,
      label,
      pointer: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      moved: false,
      target: null,
    };
    capture = event.currentTarget as HTMLElement;
    capture.addEventListener("lostpointercapture", lostCapture);
    window.addEventListener("pointermove", pointerMove, { passive: false });
    window.addEventListener("pointerup", pointerUp);
    window.addEventListener("pointercancel", cancelDrag);
    window.addEventListener("keydown", escapeDrag, true);
    window.addEventListener("blur", cancelDrag);
  }
  function startHostDrag(event: PointerEvent, host: HostProfile) {
    startDrag(event, "host", host.id, host.name);
  }
  function validTarget(key: string) {
    const current = drag.value;
    if (!current) return false;
    if (key === "all") return current.kind === "group";
    if (key === "ungrouped") return current.kind === "host";
    return (
      store.groups.some((group) => group.id === key) &&
      (current.kind !== "group" ||
        !descendantIds(store.groups, current.id).has(key))
    );
  }
  function pointerMove(event: PointerEvent) {
    const current = drag.value;
    if (!current || current.pointer !== event.pointerId) return;
    if (
      !current.moved &&
      Math.hypot(event.clientX - current.x, event.clientY - current.y) < 6
    )
      return;
    if (!current.moved) {
      current.moved = true;
      if (capture) {
        if (previewLayer.value)
          preview = createDragPreview(
            capture,
            current.x,
            current.y,
            previewLayer.value,
          );
        capture.setPointerCapture(event.pointerId);
      }
      options.closeMenu();
      window.addEventListener("click", swallowClick, true);
      suppressClick = true;
    }
    event.preventDefault();
    const element = document
      .elementFromPoint(event.clientX, event.clientY)
      ?.closest<HTMLElement>("[data-host-group-target]");
    const target =
      element && panel.value?.contains(element)
        ? (element.dataset.hostGroupTarget ?? null)
        : null;
    if (current.target !== target) {
      clearTimeout(hoverTimer);
      current.target = target && validTarget(target) ? target : null;
      if (current.target && !["all", "ungrouped"].includes(current.target))
        hoverTimer = setTimeout(() => reveal(current.target), 600);
    }
    const targetLabel =
      current.target === "all"
        ? t("顶层")
        : current.target === "ungrouped"
          ? t("未分组")
          : store.groups.find((group) => group.id === current.target)?.name;
    preview?.move(event.clientX, event.clientY, targetLabel ?? null);
    // 指针靠近树边缘时滚动真实容器，折叠的目标悬停展开。
    const scroll = panel.value?.querySelector<HTMLElement>(
      ".n-scrollbar-container",
    );
    if (scroll) {
      const rect = scroll.getBoundingClientRect();
      if (event.clientX >= rect.left && event.clientX <= rect.right) {
        if (event.clientY < rect.top + 30) scroll.scrollTop -= 16;
        else if (event.clientY > rect.bottom - 30) scroll.scrollTop += 16;
      }
    }
  }
  function swallowClick(event: MouseEvent) {
    if (suppressClick) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  }
  function finishDrag() {
    clearTimeout(hoverTimer);
    preview?.dispose();
    preview = undefined;
    const source = capture;
    const pointer = drag.value?.pointer;
    capture = undefined;
    drag.value = undefined;
    source?.removeEventListener("lostpointercapture", lostCapture);
    if (source && pointer !== undefined && source.hasPointerCapture(pointer))
      source.releasePointerCapture(pointer);
    window.removeEventListener("pointermove", pointerMove);
    window.removeEventListener("pointerup", pointerUp);
    window.removeEventListener("pointercancel", cancelDrag);
    window.removeEventListener("keydown", escapeDrag, true);
    window.removeEventListener("blur", cancelDrag);
    clearTimeout(suppressTimer);
    // 捕获阶段吞掉同一次松键生成的点击，下一次真实操作立即恢复。
    suppressTimer = setTimeout(() => {
      suppressClick = false;
      window.removeEventListener("click", swallowClick, true);
    }, 0);
  }
  function lostCapture(event: PointerEvent) {
    if (drag.value?.pointer === event.pointerId) cancelDrag();
  }
  function cancelDrag() {
    finishDrag();
  }
  function escapeDrag(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      cancelDrag();
    }
  }
  async function pointerUp(event: PointerEvent) {
    const current = drag.value;
    if (!current || current.pointer !== event.pointerId) return;
    if (current.moved) pointerMove(event);
    const target = current.target;
    const valid = current.moved && target !== null && validTarget(target);
    finishDrag();
    if (!valid || target === null || store.managementBusy) return;
    const parentId = ["all", "ungrouped"].includes(target) ? null : target;
    store.managementBusy = true;
    try {
      if (current.kind === "host") {
        const host = store.hosts.find((host) => host.id === current.id);
        if (host) await store.moveHost(host, parentId);
      } else {
        const group = store.groups.find((group) => group.id === current.id);
        if (group && group.parentId !== parentId)
          await store.saveGroup(
            { ...group, parentId, sortOrder: nextOrder(parentId) },
            group.revision,
          );
      }
      reveal(parentId);
    } catch (error) {
      options.onError(errorMessage(error));
    } finally {
      store.managementBusy = false;
    }
  }
  onBeforeUnmount(() => {
    finishDrag();
    clearTimeout(suppressTimer);
    window.removeEventListener("click", swallowClick, true);
  });
  return { drag, startDrag, startHostDrag, cancelDrag };
}

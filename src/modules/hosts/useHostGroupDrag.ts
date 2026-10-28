import "./hostDrag.css";
import { t } from "@/i18n";
import { onBeforeUnmount, ref, watch, type Ref } from "vue";
import type { HostProfile } from "@/modules/hosts/model";
import { errorMessage } from "@/services/desktop/client";
import { compareHostOrder } from "./ordering";
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
  selectedHostIds: () => string[];
  onHostDragStart: (ids: string[]) => void;
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
    hostIds: string[];
    orderTarget: { id: string; before: boolean } | null;
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
        "button, input, textarea, a, .n-tree-node-switcher, .selectable-text, [contenteditable]",
      )
    )
      return;
    // 从非正文区域起拖时取消浏览器默认选字；正文区域始终留给原生选区。
    if ((event.currentTarget as HTMLElement).hasAttribute("data-host-order-id"))
      event.preventDefault();
    const selected = new Set(options.selectedHostIds());
    drag.value = {
      kind,
      id,
      label,
      pointer: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      moved: false,
      target: null,
      hostIds:
        kind === "host"
          ? selected.has(id)
            ? store.orderedHosts
                .filter((host) => selected.has(host.id))
                .map((host) => host.id)
            : [id]
          : [],
      orderTarget: null,
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
      document.body.classList.add("host-dragging");
      window.getSelection()?.removeAllRanges();
      if (current.kind === "host") options.onHostDragStart(current.hostIds);
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
    const hit = document.elementFromPoint(event.clientX, event.clientY);
    const element = hit?.closest<HTMLElement>("[data-host-group-target]");
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
    clearOrderTarget();
    current.orderTarget = null;
    const page = panel.value?.closest(".hosts-page");
    if (
      current.kind === "host" &&
      store.manualOrder &&
      !current.target &&
      hit &&
      page?.contains(hit)
    ) {
      const item = hit.closest<HTMLElement>("[data-host-order-id]");
      const area = hit.closest<HTMLElement>("[data-host-drop-area]");
      if (item && !current.hostIds.includes(item.dataset.hostOrderId ?? "")) {
        const rect = item.getBoundingClientRect();
        const before = item.closest(".host-grid")
          ? event.clientX < rect.left + rect.width / 2
          : event.clientY < rect.top + rect.height / 2;
        current.orderTarget = { id: item.dataset.hostOrderId!, before };
        orderElement = item;
        item.dataset.hostDropPosition = before ? "before" : "after";
      } else if (!item && area) {
        // 卡片间隙按最近边界插入，底部空白才追加，避免一进入间隙就跳到末尾。
        const available = [
          ...area.querySelectorAll<HTMLElement>("[data-host-order-id]"),
        ].filter(
          (item) => !current.hostIds.includes(item.dataset.hostOrderId ?? ""),
        );
        const nearest = available
          .map((item) => {
            const rect = item.getBoundingClientRect();
            const dx = Math.max(
              rect.left - event.clientX,
              0,
              event.clientX - rect.right,
            );
            const dy = Math.max(
              rect.top - event.clientY,
              0,
              event.clientY - rect.bottom,
            );
            return { item, rect, distance: dx * dx + dy * dy };
          })
          .sort((a, b) => a.distance - b.distance)[0];
        if (nearest) {
          const { item, rect } = nearest;
          const before =
            event.clientY < rect.top
              ? true
              : event.clientY > rect.bottom
                ? false
                : item.closest(".host-grid")
                  ? event.clientX < rect.left + rect.width / 2
                  : event.clientY < rect.top + rect.height / 2;
          current.orderTarget = { id: item.dataset.hostOrderId!, before };
          orderElement = item;
          item.dataset.hostDropPosition = before ? "before" : "after";
        }
      }
    }
    const targetLabel =
      current.target === "all"
        ? t("hosts.groups.topLevel")
        : current.target === "ungrouped"
          ? t("hosts.groups.ungrouped")
          : store.groups.find((group) => group.id === current.target)?.name;
    preview?.move(
      event.clientX,
      event.clientY,
      current.orderTarget
        ? t(
            current.orderTarget.before
              ? "hosts.sort.before"
              : "hosts.sort.after",
            {
              name:
                store.hosts.find((host) => host.id === current.orderTarget?.id)
                  ?.name ?? "",
            },
          )
        : (targetLabel ?? null),
      current.hostIds.length,
    );
    // 指针靠近树边缘时滚动真实容器，折叠的目标悬停展开。
    const scrollRoot = current.orderTarget
      ? page?.querySelector(".host-list-scroll")
      : panel.value;
    const scroll = scrollRoot?.querySelector<HTMLElement>(
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
  let orderElement: HTMLElement | undefined;
  function clearOrderTarget() {
    orderElement?.removeAttribute("data-host-drop-position");
    orderElement = undefined;
  }
  function swallowClick(event: MouseEvent) {
    if (suppressClick) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  }
  function finishDrag() {
    document.body.classList.remove("host-dragging");
    clearOrderTarget();
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
    const orderTarget = current.orderTarget;
    const valid =
      current.moved &&
      ((target !== null && validTarget(target)) || !!orderTarget);
    finishDrag();
    if (!valid || store.managementBusy) return;
    const parentId =
      target && !["all", "ungrouped"].includes(target) ? target : null;
    store.managementBusy = true;
    try {
      if (current.kind === "host") {
        const moving = new Set(current.hostIds);
        const ordered = [...store.hosts]
          .sort(compareHostOrder)
          .map((host) => host.id);
        const rest = ordered.filter((id) => !moving.has(id));
        if (orderTarget) {
          const index = rest.indexOf(orderTarget.id);
          if (index < 0) return;
          rest.splice(
            index + (orderTarget.before ? 0 : 1),
            0,
            ...current.hostIds,
          );
          await store.arrangeHosts(rest);
        } else if (target !== null) {
          // 手动模式迁组追加到目标组末尾；最近使用模式只迁组，保留所有手动排序键。
          const groups = new Map(
            store.hosts.map((host) => [host.id, host.groupId]),
          );
          let last = -1;
          rest.forEach((id, index) => {
            if (groups.get(id) === parentId) last = index;
          });
          rest.splice(last < 0 ? rest.length : last + 1, 0, ...current.hostIds);
          await store.arrangeHosts(
            store.manualOrder ? rest : ordered,
            {
              hostIds: current.hostIds,
              groupId: parentId,
            },
            !store.manualOrder,
          );
          reveal(parentId);
        }
      } else {
        const group = store.groups.find((group) => group.id === current.id);
        if (group && group.parentId !== parentId)
          await store.saveGroup(
            { ...group, parentId, sortOrder: nextOrder(parentId) },
            group.revision,
          );
        reveal(parentId);
      }
    } catch (error) {
      options.onError(errorMessage(error));
    } finally {
      store.managementBusy = false;
    }
  }
  watch(() => store.manualOrder, cancelDrag);
  onBeforeUnmount(() => {
    finishDrag();
    clearTimeout(suppressTimer);
    window.removeEventListener("click", swallowClick, true);
  });
  return { drag, startDrag, startHostDrag, cancelDrag };
}

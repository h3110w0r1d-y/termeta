import {
  onBeforeUnmount,
  onMounted,
  shallowRef,
  watch,
  type CSSProperties,
  type Ref,
} from "vue";
import { tabInsertionAt } from "../tabInsertion";
import { edgeAtPoint } from "../layout";
import {
  useWorkbenchStore,
  type TabDragSource,
  type TabDropTarget,
} from "../store";

interface TabDragOptions {
  root: Ref<HTMLElement | undefined>;
  viewport: Ref<HTMLElement | undefined>;
  end: Ref<HTMLElement | undefined>;
  position: () => "top" | "left";
  onMoved: (tabId: string) => void;
}

interface DragHit {
  target?: TabDropTarget;
  marker?: CSSProperties;
  hoverTabId?: string;
}

/** 标签与窗格共用一个指针控制器，不解析外部文件或序列化会话。 */
export function useTabDrag(options: TabDragOptions) {
  const workbench = useWorkbenchStore();
  const insertionMarker = shallowRef<CSSProperties>();
  let pending:
    | {
        source: TabDragSource;
        pointerId: number;
        x: number;
        y: number;
        element: HTMLElement;
      }
    | undefined;
  let frame = 0;
  let suppressClick = false;
  let clickTimer: ReturnType<typeof setTimeout> | undefined;
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;
  let hoverId: string | undefined;
  let originActiveId: string | undefined;

  function hitTest(x: number, y: number): DragHit {
    const element = document.elementFromPoint(x, y);
    const source = workbench.drag?.source;
    if (!source) return {};
    const pane = element?.closest<HTMLElement>("[data-terminal-pane]");
    if (pane) {
      const id = pane.dataset.terminalPane!;
      if (!workbench.canSplit(source, id)) return {};
      const rect = pane.getBoundingClientRect();
      return {
        target: {
          kind: "pane",
          terminalId: id,
          edge: edgeAtPoint(
            x - rect.left,
            y - rect.top,
            rect.width,
            rect.height,
          ),
        },
      };
    }
    return hitTabBar(x, y, element, source);
  }

  function hitTabBar(
    x: number,
    y: number,
    element: Element | null,
    source: TabDragSource,
  ): DragHit {
    const bar = options.root.value;
    const strip = options.viewport.value;
    const end = options.end.value;
    if (!bar || !strip || !end || !element || !bar.contains(element)) return {};
    const bounds = bar.getBoundingClientRect();
    if (
      x < bounds.left ||
      x > bounds.right ||
      y < bounds.top ||
      y > bounds.bottom
    )
      return {};
    const vertical = options.position() === "left";
    const visible = strip.getBoundingClientRect();
    if (visible.width <= 0 || visible.height <= 0) return {};
    const point = vertical ? y : x;
    const start = vertical ? visible.top : visible.left;
    const finish = vertical ? visible.bottom : visible.right;
    const fixedZone = !!element.closest(".fixed-tabs");
    if (point < start && !fixedZone) return {};
    const tabs = Array.from(
      strip.querySelectorAll<HTMLElement>("[data-sort-tab]"),
      (tab) => {
        const rect = tab.getBoundingClientRect();
        return {
          id: tab.dataset.sortTab!,
          start: vertical ? rect.top : rect.left,
          end: vertical ? rect.bottom : rect.right,
        };
      },
    );
    // 固定页是动态标签栏的前置插入区；用负无穷取首个动态标签，避免滚动后误指向首个可见标签。
    const slot = tabInsertionAt(
      tabs,
      fixedZone ? -Infinity : point >= finish ? Infinity : point,
      source.kind === "tab" ? source.tabId : undefined,
    );
    const hoverTabId =
      element.closest<HTMLElement>("[data-sort-tab]")?.dataset.sortTab;
    if (!slot) return { hoverTabId };
    const endBounds = end.getBoundingClientRect();
    const endStart = vertical ? endBounds.top : endBounds.left;
    const position =
      slot.beforeGroupId === null
        ? (finish + endStart) / 2
        : Math.max(start + 1, Math.min(finish - 1, slot.position));
    // 指示线在标签栏覆盖层内绘制，既不挤动标签，也不受滚动裁剪和渐隐遮挡。
    const originX = bounds.left + bar.clientLeft;
    const originY = bounds.top + bar.clientTop;
    return {
      target: { kind: "tab-slot", beforeGroupId: slot.beforeGroupId },
      hoverTabId,
      marker: {
        left: `${(vertical ? visible.left + 3 : position - 1) - originX}px`,
        top: `${(vertical ? position - 1 : visible.top + 3) - originY}px`,
        width: `${vertical ? Math.max(0, visible.width - 6) : 2}px`,
        height: `${vertical ? 2 : Math.max(0, visible.height - 6)}px`,
      },
    };
  }

  function clearHover() {
    if (hoverTimer) clearTimeout(hoverTimer);
    hoverTimer = undefined;
    hoverId = undefined;
  }
  function updateTarget(x: number, y: number) {
    const hit = hitTest(x, y);
    workbench.dropTarget = hit.target;
    insertionMarker.value = hit.marker;
    // 悬停切页只认实际标签；空隙仍可插入，但不归属任何标签的悬停计时。
    const next =
      workbench.drag?.source.kind === "pane" &&
      hit.hoverTabId !== workbench.activeId
        ? hit.hoverTabId
        : undefined;
    if (next === hoverId) return;
    clearHover();
    if (!next) return;
    hoverId = next;
    hoverTimer = setTimeout(() => {
      if (workbench.drag && hoverId === next) workbench.activateTab(next);
      hoverTimer = undefined;
    }, 600);
  }
  function autoScroll() {
    const drag = workbench.drag;
    if (!drag) {
      cancel();
      return;
    }
    if (
      !sourceExists(drag.source) ||
      document.querySelector("[data-modal-open], [data-ui-popup-open]")
    ) {
      cancel();
      return;
    }
    const strip = options.viewport.value;
    if (strip) {
      const rect = strip.getBoundingClientRect();
      if (
        drag.x >= rect.left &&
        drag.x <= rect.right &&
        drag.y >= rect.top &&
        drag.y <= rect.bottom
      ) {
        const vertical = options.position() === "left";
        const point = vertical ? drag.y : drag.x;
        const start = vertical ? rect.top : rect.left;
        const end = vertical ? rect.bottom : rect.right;
        const delta = point < start + 28 ? -12 : point > end - 28 ? 12 : 0;
        if (vertical) strip.scrollTop += delta;
        else strip.scrollLeft += delta;
      }
    }
    updateTarget(drag.x, drag.y);
    frame = requestAnimationFrame(autoScroll);
  }

  function begin(
    event: PointerEvent,
    source: TabDragSource,
    element: HTMLElement,
  ) {
    if (
      event.button !== 0 ||
      !event.isPrimary ||
      pending ||
      workbench.drag ||
      document.querySelector("[data-modal-open], [data-ui-popup-open]")
    )
      return;
    if (!sourceExists(source)) return;
    // 新的按下清除上一次在窗口外松手留下的click抑制。
    suppressClick = false;
    if (clickTimer) clearTimeout(clickTimer);
    pending = {
      source,
      pointerId: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      element,
    };
  }
  function start(event: PointerEvent, id: string) {
    if ((event.target as HTMLElement).closest(".app-tab-close")) return;
    begin(
      event,
      { kind: "tab", tabId: id },
      event.currentTarget as HTMLElement,
    );
  }
  function startPane(event: PointerEvent) {
    const title = (event.target as HTMLElement).closest<HTMLElement>(
      "[data-drag-terminal]",
    );
    if (title)
      begin(
        event,
        { kind: "pane", terminalId: title.dataset.dragTerminal! },
        title,
      );
  }
  function sourceExists(source: TabDragSource) {
    const group =
      source.kind === "tab"
        ? workbench.groups.find((g) => g.id === source.tabId)
        : workbench.dragTerminal(source)?.group;
    return (
      !!group &&
      !workbench.tabBarItems
        .find((tab) => tab.id === group.id)
        ?.terminalIds.some((id) => workbench.closingTerminalIds.includes(id))
    );
  }
  function move(event: PointerEvent) {
    if (!pending || event.pointerId !== pending.pointerId) return;
    if (
      !sourceExists(pending.source) ||
      document.querySelector("[data-modal-open], [data-ui-popup-open]")
    ) {
      cancel();
      return;
    }
    if (!workbench.drag) {
      if (Math.hypot(event.clientX - pending.x, event.clientY - pending.y) < 6)
        return;
      // 捕获在不会移动的原标题/标签上；开始拖动前不修改任何布局。
      pending.element.setPointerCapture(event.pointerId);
      suppressClick = true;
      originActiveId = workbench.activeId;
      workbench.drag = {
        source: pending.source,
        x: event.clientX,
        y: event.clientY,
      };
      frame = requestAnimationFrame(autoScroll);
    } else {
      workbench.drag.x = event.clientX;
      workbench.drag.y = event.clientY;
    }
    event.preventDefault();
    event.stopPropagation();
    updateTarget(event.clientX, event.clientY);
  }

  function finish(event: PointerEvent) {
    if (!pending) {
      if (suppressClick) resetClickSoon();
      return;
    }
    if (event.pointerId !== pending.pointerId) return;
    let committed = false;
    let focusDestination = false;
    if (workbench.drag) {
      event.preventDefault();
      event.stopPropagation();
      const source = workbench.drag.source;
      // 松手按最新几何重新命中，避免最后一次滚动或标签尺寸变化留下旧目标。
      const { target } = hitTest(event.clientX, event.clientY);
      if (
        sourceExists(source) &&
        !document.querySelector("[data-modal-open], [data-ui-popup-open]")
      ) {
        if (target?.kind === "tab-slot") {
          const result = workbench.dropOnTabBar(source, target.beforeGroupId);
          committed = result === "reordered" || result === "detached";
          focusDestination = result === "detached";
        } else if (target?.kind === "pane") {
          committed = workbench.mergeTerminal(
            source,
            target.terminalId,
            target.edge,
          );
          focusDestination = committed;
        }
      }
    }
    const destination = workbench.activeId;
    cancel(!committed);
    if (focusDestination) options.onMoved(destination);
    resetClickSoon();
  }
  function resetClickSoon() {
    if (clickTimer) clearTimeout(clickTimer);
    clickTimer = setTimeout(() => {
      suppressClick = false;
    }, 0);
  }
  function cancel(restore = true) {
    clearHover();
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
    const previous = pending;
    pending = undefined;
    workbench.cancelDrag();
    insertionMarker.value = undefined;
    if (restore && originActiveId) workbench.activateTab(originActiveId);
    originActiveId = undefined;
    if (previous?.element.hasPointerCapture(previous.pointerId))
      previous.element.releasePointerCapture(previous.pointerId);
    // Escape/失焦时鼠标可能仍按住，抑制后续click直至松手或下次按下。
  }
  function cancelPointer(event: PointerEvent) {
    if (pending?.pointerId === event.pointerId) {
      cancel();
      if (event.type === "pointercancel") resetClickSoon();
    }
  }
  function blur() {
    cancel();
  }
  function keydown(event: KeyboardEvent) {
    if (pending && event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      cancel();
    }
  }
  function click(event: MouseEvent) {
    if (!suppressClick || event.detail === 0) return;
    if (
      !(event.target as HTMLElement).closest("[data-drag-terminal], .app-tabs")
    )
      return;
    event.preventDefault();
    event.stopPropagation();
  }
  // 布局方向改变时结束旧手势，避免沿新坐标轴提交原拖动。
  watch(options.position, () => {
    if (pending) cancel();
  });
  onMounted(() => {
    document.addEventListener("pointerdown", startPane);
    document.addEventListener("pointermove", move, {
      capture: true,
      passive: false,
    });
    document.addEventListener("pointerup", finish, true);
    document.addEventListener("pointercancel", cancelPointer, true);
    document.addEventListener("lostpointercapture", cancelPointer, true);
    document.addEventListener("keydown", keydown, true);
    document.addEventListener("click", click, true);
    document.addEventListener("dblclick", click, true);
    window.addEventListener("blur", blur);
  });
  onBeforeUnmount(() => {
    cancel();
    if (clickTimer) clearTimeout(clickTimer);
    document.removeEventListener("pointerdown", startPane);
    document.removeEventListener("pointermove", move, true);
    document.removeEventListener("pointerup", finish, true);
    document.removeEventListener("pointercancel", cancelPointer, true);
    document.removeEventListener("lostpointercapture", cancelPointer, true);
    document.removeEventListener("keydown", keydown, true);
    document.removeEventListener("click", click, true);
    document.removeEventListener("dblclick", click, true);
    window.removeEventListener("blur", blur);
  });
  return { start, insertionMarker };
}

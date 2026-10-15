import { onBeforeUnmount, onMounted } from "vue";
import { edgeAtPoint } from "./layout";
import {
  useWorkspace,
  type WorkspaceDragSource,
  type WorkspaceDropTarget,
} from "./store";

/** 标签与窗格共用一个指针控制器，不解析外部文件或序列化会话。 */
export function useTabDrag(onMoved: (tabId: string) => void) {
  const workspace = useWorkspace();
  let pending:
    | {
        source: WorkspaceDragSource;
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

  function hitTest(x: number, y: number): WorkspaceDropTarget | undefined {
    const element = document.elementFromPoint(x, y);
    const source = workspace.drag?.source;
    if (!source) return;
    const pane = element?.closest<HTMLElement>("[data-terminal-pane]");
    if (pane) {
      const id = pane.dataset.terminalPane!;
      if (!workspace.canSplit(source, id)) return;
      const rect = pane.getBoundingClientRect();
      return {
        kind: "pane",
        terminalId: id,
        edge: edgeAtPoint(x - rect.left, y - rect.top, rect.width, rect.height),
      };
    }
    const tab = element?.closest<HTMLElement>("[data-sort-tab]");
    if (tab) {
      if (source.kind === "tab" && tab.dataset.sortTab === source.tabId) return;
      const rect = tab.getBoundingClientRect();
      const vertical = !!tab.closest(".tabs-left");
      return {
        kind: "tab",
        tabId: tab.dataset.sortTab!,
        before: vertical
          ? y < rect.top + rect.height / 2
          : x < rect.left + rect.width / 2,
      };
    }
    // 末尾是整片几何区域，不依赖按钮透传属性或零高度的flex空白元素。
    // 用可见会话区裁剪末项边界，溢出时仍可在新建按钮前显示插入位置。
    const bar = document.querySelector<HTMLElement>(".app-tabs");
    const strip = bar?.querySelector<HTMLElement>(".session-tabs");
    const last = strip?.querySelector<HTMLElement>(
      "[data-sort-tab]:last-child",
    );
    if (!bar || !strip || !last) return;
    const bounds = bar.getBoundingClientRect();
    if (
      x < bounds.left ||
      x > bounds.right ||
      y < bounds.top ||
      y > bounds.bottom
    )
      return;
    const visible = strip.getBoundingClientRect();
    const end = last.getBoundingClientRect();
    const vertical = !!bar.closest(".tabs-left");
    if (
      vertical
        ? y >= Math.min(end.bottom, visible.bottom)
        : x >= Math.min(end.right, visible.right)
    )
      return { kind: "tab-end" };
  }

  function clearHover() {
    if (hoverTimer) clearTimeout(hoverTimer);
    hoverTimer = undefined;
    hoverId = undefined;
  }
  function updateTarget(x: number, y: number) {
    const target = hitTest(x, y);
    workspace.dropTarget = target;
    // 窗格拖动悬停其他标签后切换目标；松在标签上仍表示拆成独立Tab。
    const next =
      workspace.drag?.source.kind === "pane" &&
      target?.kind === "tab" &&
      target.tabId !== workspace.activeId
        ? target.tabId
        : undefined;
    if (next === hoverId) return;
    clearHover();
    if (!next) return;
    hoverId = next;
    hoverTimer = setTimeout(() => {
      if (workspace.drag && hoverId === next) workspace.activateTab(next);
      hoverTimer = undefined;
    }, 600);
  }
  function autoScroll() {
    const drag = workspace.drag;
    if (!drag) return;
    if (
      !sourceExists(drag.source) ||
      document.querySelector("[data-modal-open], [data-ui-popup-open]")
    ) {
      cancel();
      return;
    }
    const strip = document.querySelector<HTMLElement>(".session-tabs");
    if (strip) {
      const rect = strip.getBoundingClientRect();
      if (
        drag.x >= rect.left &&
        drag.x <= rect.right &&
        drag.y >= rect.top &&
        drag.y <= rect.bottom
      ) {
        const vertical = !!strip.closest(".tabs-left");
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
    source: WorkspaceDragSource,
    element: HTMLElement,
  ) {
    if (
      event.button !== 0 ||
      !event.isPrimary ||
      pending ||
      workspace.drag ||
      document.querySelector("[data-modal-open], [data-ui-popup-open]")
    )
      return;
    const group =
      source.kind === "pane"
        ? workspace.dragTerminal(source)?.group
        : workspace.groups.find((g) => g.id === source.tabId);
    if (
      !group ||
      workspace.workspaceTabs
        .find((tab) => tab.id === group.id)
        ?.terminalIds.some((id) => workspace.closingTerminalIds.includes(id))
    )
      return;
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
    if ((event.target as HTMLElement).closest(".workspace-tab-close")) return;
    const element = (event.target as HTMLElement).closest<HTMLElement>(
      '[role="tab"]',
    );
    if (element) begin(event, { kind: "tab", tabId: id }, element);
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
  function sourceExists(source: WorkspaceDragSource) {
    const group =
      source.kind === "tab"
        ? workspace.groups.find((g) => g.id === source.tabId)
        : workspace.dragTerminal(source)?.group;
    return (
      !!group &&
      !workspace.workspaceTabs
        .find((tab) => tab.id === group.id)
        ?.terminalIds.some((id) => workspace.closingTerminalIds.includes(id))
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
    if (!workspace.drag) {
      if (Math.hypot(event.clientX - pending.x, event.clientY - pending.y) < 6)
        return;
      // 捕获在不会移动的原标题/标签上；开始拖动前不修改任何布局。
      pending.element.setPointerCapture(event.pointerId);
      suppressClick = true;
      originActiveId = workspace.activeId;
      workspace.drag = {
        source: pending.source,
        x: event.clientX,
        y: event.clientY,
      };
      frame = requestAnimationFrame(autoScroll);
    } else {
      workspace.drag.x = event.clientX;
      workspace.drag.y = event.clientY;
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
    let moved = false;
    if (workspace.drag) {
      event.preventDefault();
      event.stopPropagation();
      const source = workspace.drag.source;
      const target = hitTest(event.clientX, event.clientY);
      if (!document.querySelector("[data-modal-open], [data-ui-popup-open]")) {
        if (target?.kind === "tab" || target?.kind === "tab-end")
          moved = workspace.dropOnTabBar(
            source,
            target.kind === "tab" ? target : undefined,
          );
        else if (target?.kind === "pane")
          moved = workspace.mergeTerminal(
            source,
            target.terminalId,
            target.edge,
          );
      }
    }
    const destination = workspace.activeId;
    cancel(!moved);
    if (moved) onMoved(destination);
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
    workspace.cancelDrag();
    if (restore && originActiveId) workspace.activateTab(originActiveId);
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
    window.removeEventListener("blur", blur);
  });
  return { start };
}

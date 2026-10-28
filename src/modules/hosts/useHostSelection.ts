import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
  type Ref,
} from "vue";
import type { HostProfile } from "@/modules/hosts/model";

// 选区只属于当前浏览范围；锚点与键盘焦点分开，Shift 可连续扩展同一范围。
export function useHostSelection(
  visible: Ref<HostProfile[]>,
  root: Ref<HTMLElement | undefined>,
) {
  const selectedIds = ref<string[]>([]);
  const anchorId = ref<string>();
  const focusedId = ref<string>();
  const focusWithin = ref(false);
  const windowFocused = ref(document.hasFocus());
  const hasFocus = computed(() => windowFocused.value && focusWithin.value);
  function syncFocus() {
    focusWithin.value = !!root.value?.contains(document.activeElement);
  }
  function focusOut(event: FocusEvent) {
    focusWithin.value =
      event.relatedTarget instanceof Node &&
      !!root.value?.contains(event.relatedTarget);
  }
  function focusRoot() {
    root.value?.focus({ preventScroll: true });
    syncFocus();
  }
  function windowFocus() {
    windowFocused.value = true;
    syncFocus();
  }
  function windowBlur() {
    windowFocused.value = false;
  }
  watch(
    root,
    (element, _, onCleanup) => {
      syncFocus();
      element?.addEventListener("focusin", syncFocus);
      element?.addEventListener("focusout", focusOut);
      onCleanup(() => {
        element?.removeEventListener("focusin", syncFocus);
        element?.removeEventListener("focusout", focusOut);
      });
    },
    { flush: "post" },
  );
  onMounted(() => {
    window.addEventListener("focus", windowFocus);
    window.addEventListener("blur", windowBlur);
  });
  onBeforeUnmount(() => {
    window.removeEventListener("focus", windowFocus);
    window.removeEventListener("blur", windowBlur);
  });
  const selectedHosts = computed(() => {
    const ids = new Set(selectedIds.value);
    return visible.value.filter((host) => ids.has(host.id));
  });
  function clear() {
    selectedIds.value = [];
    anchorId.value = undefined;
    focusedId.value = undefined;
  }
  function replace(ids: string[]) {
    const selected = new Set(ids);
    selectedIds.value = visible.value
      .filter((host) => selected.has(host.id))
      .map((host) => host.id);
    if (!anchorId.value || !selected.has(anchorId.value))
      anchorId.value = selectedIds.value[0];
    if (!focusedId.value || !selected.has(focusedId.value))
      focusedId.value = selectedIds.value[0];
  }
  function element(id: string) {
    return [
      ...(root.value?.querySelectorAll<HTMLElement>("[data-host-order-id]") ??
        []),
    ].find((item) => item.dataset.hostOrderId === id);
  }
  async function focus(id: string) {
    focusedId.value = id;
    await nextTick();
    element(id)?.focus({ preventScroll: true });
    element(id)?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }
  function select(id: string, extend = false, toggle = false) {
    const ids = visible.value.map((host) => host.id);
    const start = ids.indexOf(anchorId.value ?? "");
    const end = ids.indexOf(id);
    if (end < 0) return;
    if (extend && start >= 0) {
      const range = ids.slice(Math.min(start, end), Math.max(start, end) + 1);
      selectedIds.value = toggle
        ? [...new Set([...selectedIds.value, ...range])]
        : range;
    } else {
      selectedIds.value = toggle
        ? selectedIds.value.includes(id)
          ? selectedIds.value.filter((value) => value !== id)
          : [...selectedIds.value, id]
        : [id];
      anchorId.value = id;
    }
    void focus(id);
  }
  function click(event: MouseEvent, host: HostProfile) {
    if (
      (event.target as HTMLElement).closest(
        "button, .selectable-text, a, input",
      )
    )
      return;
    window.getSelection()?.removeAllRanges();
    select(host.id, event.shiftKey, event.ctrlKey || event.metaKey);
  }
  function beginDrag(ids: string[]) {
    const available = new Set(visible.value.map((host) => host.id));
    selectedIds.value = ids.filter((id) => available.has(id));
    if (!anchorId.value || !selectedIds.value.includes(anchorId.value))
      anchorId.value = selectedIds.value[0];
    focusedId.value = selectedIds.value[0];
    // 拖动不滚屏，但结束后回车仍交给卡片；侧栏不可见的来源不抢焦点。
    if (focusedId.value)
      element(focusedId.value)?.focus({ preventScroll: true });
  }
  function first() {
    const host = visible.value[0];
    if (host) select(host.id);
  }
  function navigate(event: KeyboardEvent, host?: HostProfile) {
    if (
      !event.key.startsWith("Arrow") ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey
    )
      return false;
    const keys = ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"];
    if (!keys.includes(event.key)) return false;
    event.preventDefault();
    event.stopPropagation();
    host ??=
      selectedHosts.value.find((item) => item.id === focusedId.value) ??
      selectedHosts.value[0];
    if (!host) {
      first();
      return true;
    }
    const currentId = host.id;
    const index = visible.value.findIndex((item) => item.id === currentId);
    let target: HostProfile | undefined;
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      target = visible.value[index + (event.key === "ArrowLeft" ? -1 : 1)];
    } else {
      // 以实际矩形寻找相邻行，适应卡片响应式列数、行高和列表模式。
      const elements = new Map(
        [
          ...(root.value?.querySelectorAll<HTMLElement>(
            "[data-host-order-id]",
          ) ?? []),
        ].map((item) => [item.dataset.hostOrderId, item]),
      );
      const rect = elements.get(host.id)?.getBoundingClientRect();
      if (rect) {
        const candidates = visible.value
          .flatMap((item) => {
            const box = elements.get(item.id)?.getBoundingClientRect();
            if (
              !box ||
              (event.key === "ArrowUp"
                ? box.top >= rect.top - 1
                : box.top <= rect.top + 1)
            )
              return [];
            return [
              {
                item,
                row: Math.abs(box.top - rect.top),
                column: Math.abs(
                  box.left + box.width / 2 - rect.left - rect.width / 2,
                ),
              },
            ];
          })
          .sort((a, b) => a.row - b.row || a.column - b.column);
        const nearestRow = candidates[0]?.row;
        target = candidates
          .filter(
            (candidate) => Math.abs(candidate.row - (nearestRow ?? 0)) < 1,
          )
          .sort((a, b) => a.column - b.column)[0]?.item;
      }
    }
    if (target) select(target.id, event.shiftKey);
    return true;
  }
  watch(
    () => visible.value.map((host) => host.id),
    (ids) => {
      const available = new Set(ids);
      selectedIds.value = selectedIds.value.filter((id) => available.has(id));
      if (anchorId.value && !available.has(anchorId.value))
        anchorId.value = undefined;
      if (focusedId.value && !available.has(focusedId.value))
        focusedId.value = undefined;
    },
  );
  return {
    selectedIds,
    selectedHosts,
    focusedId,
    hasFocus,
    focusRoot,
    replace,
    clear,
    select,
    click,
    beginDrag,
    first,
    navigate,
  };
}

<script setup lang="ts">
import { computed, h, onBeforeUnmount, ref, watch } from "vue";
import {
  NDataTable,
  type DataTableColumns,
  type DataTableInst,
  type DataTableRowKey,
  type DataTableSortState,
} from "naive-ui";
import { t } from "@/i18n";
import type { FileEntry } from "@/services/desktop/types";
import { directoryRows, entryLabel, isDirectory } from "./fileEntries";
import FileEntryIcon from "./FileEntryIcon.vue";
import { formatFileSize } from "./format";

const props = defineProps<{
  entries: FileEntry[];
  showHidden: boolean;
  disabled: boolean;
  loading: boolean;
  atRoot: boolean;
}>();
const selected = defineModel<string[]>("selected", { required: true });
const emit = defineEmits<{
  navigate: [row: FileEntry];
  drag: [event: PointerEvent, entries: FileEntry[]];
  contextmenu: [event: MouseEvent, row: FileEntry];
}>();
const root = ref<HTMLElement>();
const table = ref<DataTableInst>();
const order = ref<DataTableSortState["order"]>("ascend");
const rows = computed(() =>
  directoryRows(props.entries, props.showHidden, order.value),
);
const selectable = computed(() =>
  rows.value.filter((row) => row.name !== ".."),
);
const selectedSet = computed(() => new Set(selected.value));
const unavailable = computed(() => props.disabled || props.loading);
const anchor = ref<string>();
const focused = ref<string>();
const rowHeight = 36;
const columns = computed<DataTableColumns<FileEntry>>(() => [
  {
    type: "selection",
    width: 36,
    disabled: (row) => row.name === ".." || unavailable.value,
  },
  {
    title: t("common.fields.name"),
    key: "name",
    minWidth: 160,
    className: "sftp-name-cell",
    sorter: true,
    sortOrder: order.value,
    render: (row) =>
      h("span", { class: "sftp-file-name", title: entryLabel(row) }, [
        h(FileEntryIcon, { kind: row.kind, targetKind: row.targetKind }),
        h("span", { class: "sftp-file-label" }, entryLabel(row)),
      ]),
  },
  {
    title: t("files.fields.size"),
    key: "size",
    width: 96,
    align: "right",
    render: (row) => formatFileSize(row.size),
  },
  {
    title: t("files.fields.permissions"),
    key: "permissions",
    width: 64,
    render: (row) =>
      row.permissions === null
        ? "—"
        : (row.permissions & 0o7777).toString(8).padStart(3, "0"),
  },
]);

function updateSort(sort: DataTableSortState | DataTableSortState[] | null) {
  cancelGesture(true);
  order.value = (Array.isArray(sort) ? sort[0] : sort)?.order ?? false;
}
function updateChecked(keys: DataTableRowKey[]) {
  if (unavailable.value) return;
  const valid = new Set(selectable.value.map((row) => row.name));
  const next = keys.filter(
    (key): key is string => typeof key === "string" && valid.has(key),
  );
  const changed = next.find((key) => !selectedSet.value.has(key));
  if (changed) anchor.value = focused.value = changed;
  selected.value = next;
}
function selectRow(row: FileEntry, event: MouseEvent | KeyboardEvent) {
  if (row.name === ".." || unavailable.value) return;
  focused.value = row.name;
  if (event.shiftKey) {
    const names = selectable.value.map((item) => item.name);
    const start = names.indexOf(anchor.value ?? row.name);
    const end = names.indexOf(row.name);
    const range = names.slice(
      Math.min(Math.max(start, 0), end),
      Math.max(start, end) + 1,
    );
    selected.value =
      event.ctrlKey || event.metaKey
        ? [...new Set([...selected.value, ...range])]
        : range;
    anchor.value ??= row.name;
  } else {
    anchor.value = row.name;
    selected.value =
      event.ctrlKey || event.metaKey
        ? selectedSet.value.has(row.name)
          ? selected.value.filter((name) => name !== row.name)
          : [...selected.value, row.name]
        : [row.name];
  }
}
function openRow(row: FileEntry) {
  if (
    unavailable.value ||
    !isDirectory(row) ||
    (props.atRoot && row.name === "..")
  )
    return;
  emit("navigate", row);
}
function interactiveTarget(target: EventTarget | null) {
  return (
    target instanceof Element &&
    !!target.closest(".n-checkbox, button, input, a")
  );
}
let suppressClick = false;
let nameGesture: { id: number; x: number; y: number } | undefined;

function startNameGesture(event: PointerEvent, row: FileEntry) {
  if (
    !event.isPrimary ||
    event.button !== 0 ||
    unavailable.value ||
    interactiveTarget(event.target)
  )
    return;
  suppressClick = false;
  focusTable();
  if (
    event.shiftKey ||
    event.ctrlKey ||
    event.metaKey ||
    !selectedSet.value.has(row.name)
  )
    selectRow(row, event);
  focused.value = row.name;
  if (row.name === ".." || event.shiftKey || event.ctrlKey || event.metaKey)
    return;
  // 名称拖动的捕获与阈值由调用方管理，这里只阻止松手后的误点击。
  nameGesture = { id: event.pointerId, x: event.clientX, y: event.clientY };
  watchPointer();
  emit(
    "drag",
    event,
    selectable.value.filter((item) => selectedSet.value.has(item.name)),
  );
}
function rowProps(row: FileEntry, index: number) {
  const parentDisabled = row.name === ".." && props.atRoot;
  return {
    "data-sftp-row-index": index,
    "aria-selected": row.name !== ".." && selectedSet.value.has(row.name),
    "aria-disabled": unavailable.value || parentDisabled,
    style: { height: `${rowHeight}px`, opacity: parentDisabled ? 0.4 : 1 },
    onPointerdown: (event: PointerEvent) => {
      if ((event.target as Element).closest(".sftp-name-cell"))
        startNameGesture(event, row);
    },
    onClick: (event: MouseEvent) => {
      if (suppressClick || interactiveTarget(event.target) || unavailable.value)
        return;
      // 修饰键在名称按下时已处理，避免切换两次。
      if (
        (event.target as Element).closest(".sftp-name-cell") &&
        (event.shiftKey || event.ctrlKey || event.metaKey)
      )
        return;
      selectRow(row, event);
    },
    onDblclick: (event: MouseEvent) => {
      if (!suppressClick && !interactiveTarget(event.target)) openRow(row);
    },
    onContextmenu: (event: MouseEvent) => {
      event.preventDefault();
      if (unavailable.value || row.name === "..") return;
      cancelGesture(true);
      focusTable();
      if (!selectedSet.value.has(row.name)) selected.value = [row.name];
      anchor.value = focused.value = row.name;
      emit("contextmenu", event, row);
    },
  };
}
function focusTable() {
  root.value?.focus({ preventScroll: true });
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
  suppressClick = false;
  if (
    interactiveTarget(event.target) ||
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
    Math.hypot(event.clientX - nameGesture.x, event.clientY - nameGesture.y) >=
      6
  )
    suppressClick = true;
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
    suppressClick = true;
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
    anchor.value = selected.value[0];
    focused.value = selected.value[selected.value.length - 1];
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
  if (suppressClick) {
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
function keydown(event: KeyboardEvent) {
  if (event.key === "Escape" && (box || nameGesture)) {
    event.preventDefault();
    cancelGesture(true);
    return;
  }
  if (unavailable.value || interactiveTarget(event.target)) return;
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "a") {
    event.preventDefault();
    selected.value = selectable.value.map((row) => row.name);
    anchor.value = selected.value[0];
    return;
  }
  if (event.key === "Escape") {
    selected.value = [];
    return;
  }
  const index = rows.value.findIndex((row) => row.name === focused.value);
  if (event.key === " " && index >= 0) {
    const row = rows.value[index];
    if (!row || row.name === "..") return;
    event.preventDefault();
    if (event.shiftKey) selectRow(row, event);
    else {
      anchor.value = row.name;
      selected.value = selectedSet.value.has(row.name)
        ? selected.value.filter((name) => name !== row.name)
        : [...selected.value, row.name];
    }
    return;
  }
  if (event.key === "Enter") {
    const row = rows.value[index];
    if (row) {
      event.preventDefault();
      openRow(row);
    }
    return;
  }
  const next =
    event.key === "ArrowDown"
      ? Math.min(rows.value.length - 1, index + 1)
      : event.key === "ArrowUp"
        ? Math.max(0, index < 0 ? 0 : index - 1)
        : event.key === "Home"
          ? 0
          : event.key === "End"
            ? rows.value.length - 1
            : -1;
  if (next < 0) return;
  const row = rows.value[next];
  if (!row) return;
  event.preventDefault();
  focused.value = row.name;
  if ((!event.ctrlKey && !event.metaKey) || event.shiftKey)
    selectRow(row, event);
  table.value?.scrollTo({ index: next });
}
watch(
  () => [props.entries, props.showHidden] as const,
  () => {
    cancelGesture(false);
    const valid = new Set(selectable.value.map((row) => row.name));
    const next = selected.value.filter((name) => valid.has(name));
    if (next.length !== selected.value.length) selected.value = next;
    if (anchor.value && !valid.has(anchor.value)) anchor.value = undefined;
    if (focused.value && focused.value !== ".." && !valid.has(focused.value))
      focused.value = undefined;
  },
);
watch(unavailable, (value) => {
  if (value) cancelGesture(false);
});
onBeforeUnmount(() => cancelGesture(false));
</script>

<template>
  <div
    ref="root"
    class="sftp-file-table"
    role="group"
    tabindex="0"
    :aria-label="t('files.table.label')"
    :aria-disabled="unavailable"
    @pointerdown="startBox"
    @lostpointercapture="lostCapture"
    @click.capture="captureClick"
    @dblclick.capture="suppressClick && $event.stopPropagation()"
    @keydown="keydown"
  >
    <NDataTable
      ref="table"
      class="sftp-file-data"
      size="small"
      flex-height
      virtual-scroll
      remote
      :columns="columns"
      :data="rows"
      :checked-row-keys="selected"
      :row-key="(row: FileEntry) => row.name"
      :row-props="rowProps"
      :row-class-name="
        (row: FileEntry) =>
          selectedSet.has(row.name) ? 'sftp-file-selected' : ''
      "
      :loading="loading"
      :scroll-x="374"
      :min-row-height="rowHeight"
      :single-line="false"
      :bordered="false"
      @update:checked-row-keys="updateChecked"
      @update:sorter="updateSort"
    />
    <div
      v-if="rectangle"
      class="sftp-selection-rectangle"
      aria-hidden="true"
      :style="{
        left: rectangle.left + 'px',
        top: rectangle.top + 'px',
        width: rectangle.width + 'px',
        height: rectangle.height + 'px',
      }"
    />
  </div>
</template>

<style scoped>
.sftp-file-table {
  position: relative;
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 100px;
  overflow: hidden;
  outline: none;
  user-select: none;
}
.sftp-file-data {
  flex: 1;
  min-width: 0;
  min-height: 0;
}
:deep(.n-data-table-th__title) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sftp-selection-rectangle {
  position: absolute;
  z-index: 4;
  border: 1px solid var(--accent);
  background: color-mix(in srgb, var(--accent) 15%, transparent);
  pointer-events: none;
}
:deep(.sftp-file-name) {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  cursor: default;
}
:deep(.sftp-file-name .n-icon) {
  flex-shrink: 0;
}
:deep(.sftp-file-label) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
:deep([data-sftp-row-index] .n-data-table-td) {
  box-sizing: border-box;
  height: 36px;
  padding-top: 0;
  padding-bottom: 0;
  white-space: nowrap;
}
:deep(.sftp-file-selected .n-data-table-td) {
  background: color-mix(in srgb, var(--accent) 12%, var(--n-td-color));
}
</style>

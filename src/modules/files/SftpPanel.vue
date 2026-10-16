<script setup lang="ts">
import { validateForm } from "../../services/forms/rules";
import { computed, h, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NCheckbox,
  NDataTable,
  NFlex,
  NInput,
  NScrollbar,
  NText,
  type DataTableColumns,
  type DataTableRowKey,
  type DataTableSortState,
  NForm,
  NFormItem,
  type FormInst,
} from "naive-ui";
import { desktop, errorMessage } from "../../services/desktop/client";
import type {
  FileEntry,
  SftpLocalDrop,
  SftpDropPosition,
} from "../../services/desktop/types";
import { useWorkspace } from "../workspace/store";
import type { TerminalTab } from "../workspace/store";
import { confirmAction } from "../../components/ui/confirmation";
import { directoryRows, entryLabel, isDirectory } from "./fileEntries";
import { useSftpTransferQueue } from "./useSftpTransferQueue";
import FileEntryIcon from "./FileEntryIcon.vue";
import { formatFileSize } from "./format";
import { useSettings } from "../settings/store";
const props = defineProps<{ tab: TerminalTab }>();
const panel = ref<HTMLElement>();
const workspace = useWorkspace();
const panelWidth = ref(380);
let resizeStart: { x: number; width: number; pointerId: number } | undefined;
function startResize(event: PointerEvent) {
  if (event.button !== 0 || !event.isPrimary) return;
  event.preventDefault();
  resizeStart = {
    x: event.clientX,
    width: panel.value?.getBoundingClientRect().width ?? panelWidth.value,
    pointerId: event.pointerId,
  };
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}
function moveResize(event: PointerEvent) {
  if (!resizeStart || event.pointerId !== resizeStart.pointerId) return;
  const available = panel.value?.parentElement?.clientWidth ?? 700;
  panelWidth.value = Math.max(
    240,
    Math.min(
      available - 120,
      resizeStart.width + resizeStart.x - event.clientX,
    ),
  );
}
function stopResize(event: PointerEvent) {
  resizeStart = undefined;
  const element = event.currentTarget as HTMLElement;
  if (element.hasPointerCapture(event.pointerId))
    element.releasePointerCapture(event.pointerId);
}
function resizeKey(event: KeyboardEvent) {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  event.preventDefault();
  const available = panel.value?.parentElement?.clientWidth ?? 700;
  panelWidth.value = Math.max(
    240,
    Math.min(
      available - 120,
      panelWidth.value + (event.key === "ArrowLeft" ? 20 : -20),
    ),
  );
}
const directory = ref(".");
const pathInput = ref(".");
const pathForm = ref<FormInst>();
const pathRules = {
  path: {
    required: true,
    message: "请输入远端路径",
    trigger: ["input", "blur"],
  },
};
async function navigatePath() {
  if (await validateForm(pathForm.value)) await load(pathInput.value);
}
const entries = ref<FileEntry[]>([]);
const checkedNames = ref<DataTableRowKey[]>([]);
const selectedEntries = computed(() =>
  entries.value.filter((entry) => checkedNames.value.includes(entry.name)),
);
const selected = computed(() =>
  selectedEntries.value.length === 1 ? selectedEntries.value[0] : undefined,
);
const nameSort = ref<DataTableSortState["order"]>("ascend");
const atRoot = computed(() => directory.value === "/");
const settings = useSettings();
const hidden = computed(() => settings.value.sftpShowHidden);
async function updateHidden(show: boolean) {
  try {
    await settings.setSftpShowHidden(show);
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}
const loading = ref(false);
const hasMore = ref(false);
const error = ref("");
const notice = ref("");
const {
  tasks,
  visibleTasks,
  visibleTransfer,
  transferring,
  progress,
  enqueue,
  cancelQueued,
  cancel,
  cancelActive,
} = useSftpTransferQueue({
  tab: () => props.tab,
  busy: () => busy.value,
  disposed: () => disposed,
  connected: () => connected.value,
  sameSession,
  refresh: (path) => load(path),
  releaseDrag: () => {
    releaseFilePointer();
    suppressClick = false;
  },
  error,
});

const dropTarget = ref<string>();
const consumedDrops = new Set<string>();
const eventUnlisteners: (() => void)[] = [];
let pendingDrop = false;
let dragStart:
  { x: number; y: number; names: string[]; directories: string[] } | undefined;
let suppressClick = false;
let filePointer: { element: HTMLElement; id: number } | undefined;
function releaseFilePointer() {
  const pointer = filePointer;
  filePointer = undefined;
  dragStart = undefined;
  if (pointer?.element.hasPointerCapture(pointer.id))
    pointer.element.releasePointerCapture(pointer.id);
}
const listSession = ref<string>();
const connected = computed(
  () => props.tab.state === "connected" && !!props.tab.sessionId,
);
const stale = computed(
  () => !connected.value || listSession.value !== props.tab.sessionId,
);
const busy = computed(() => loading.value || transferring.value);
const rows = computed(() =>
  directoryRows(entries.value, hidden.value, nameSort.value),
);
function updateSort(sort: DataTableSortState | DataTableSortState[] | null) {
  nameSort.value = (Array.isArray(sort) ? sort[0] : sort)?.order ?? false;
}
const columns = computed<DataTableColumns<FileEntry>>(() => [
  {
    type: "selection",
    width: 36,
    disabled: (row) => row.name === ".." || busy.value || stale.value,
  },
  {
    title: "名称",
    key: "name",
    minWidth: 150,
    sorter: true,
    sortOrder: nameSort.value,
    render: (row) =>
      h("span", { class: "sftp-entry-name", title: entryLabel(row) }, [
        h(FileEntryIcon, { kind: row.kind, targetKind: row.targetKind }),
        h("span", { class: "sftp-entry-label" }, entryLabel(row)),
      ]),
  },
  {
    title: "大小",
    key: "size",
    width: 100,
    render: (row) => formatFileSize(row.size),
  },
  {
    title: "权限",
    key: "permissions",
    width: 70,
    render: (row) =>
      row.permissions === null
        ? "—"
        : (row.permissions & 0o7777).toString(8).padStart(3, "0"),
  },
]);
let generation = 0;
let disposed = false;
async function load(path = directory.value, more = false) {
  const sessionId = props.tab.sessionId;
  if (!connected.value || !sessionId || busy.value) return;
  const current = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const page = await desktop.sftpList(sessionId, path, more);
    if (disposed || current !== generation || sessionId !== props.tab.sessionId)
      return;
    directory.value = pathInput.value = page.path;
    entries.value = more ? [...entries.value, ...page.entries] : page.entries;
    hasMore.value = page.hasMore;
    listSession.value = sessionId;
    if (!more) checkedNames.value = [];
  } catch (cause) {
    if (!disposed && current === generation) {
      error.value = errorMessage(cause);
      listSession.value = undefined;
    }
  } finally {
    if (current === generation) loading.value = false;
  }
}
function sameSession(sessionId: string | undefined) {
  return (
    !disposed &&
    connected.value &&
    sessionId === props.tab.sessionId &&
    listSession.value === sessionId
  );
}
async function transfer(upload: boolean) {
  if (
    stale.value ||
    loading.value ||
    (!upload && selected.value?.kind !== "file")
  )
    return;
  enqueue({
    sessionId: props.tab.sessionId!,
    path: directory.value,
    upload,
    name: upload ? null : selected.value!.name,
  });
}
function targetAt(position: SftpDropPosition): string | undefined {
  if (
    !props.tab.sftpOpen ||
    workspace.active?.terminalId !== props.tab.terminalId ||
    stale.value ||
    dragStart !== undefined ||
    tasks.value.some((task) => task.dragNames && task.status === "running")
  )
    return;
  // 原生桥已统一为WebView逻辑坐标，不能再次按Retina倍率缩小。
  const { x, y } = position;
  const bounds = panel.value?.getBoundingClientRect();
  if (
    !bounds ||
    x < bounds.left ||
    x > bounds.right ||
    y < bounds.top ||
    y > bounds.bottom ||
    !bounds.width ||
    !bounds.height
  )
    return;
  return directory.value;
}
async function receiveDrop(drop: SftpLocalDrop) {
  const target = targetAt(drop.position);
  dropTarget.value = undefined;
  if (!target) return;
  if (drop.error) {
    error.value = drop.error;
    return;
  }
  if (consumedDrops.has(drop.dropId) || pendingDrop) return;
  consumedDrops.add(drop.dropId);
  if (consumedDrops.size > 256)
    consumedDrops.delete(consumedDrops.values().next().value!);
  pendingDrop = true;
  const sessionId = props.tab.sessionId!;
  try {
    if (
      await confirmAction(
        `将拖入的文件或目录上传到 ${target}？同名目录合并；已有文件会在覆盖前逐项询问。`,
      )
    ) {
      if (sameSession(sessionId))
        enqueue({
          sessionId,
          path: target,
          upload: true,
          name: null,
          dropId: drop.dropId,
        });
    }
  } finally {
    pendingDrop = false;
  }
}
function startFileDrag(event: PointerEvent, row: FileEntry) {
  // 新的按下属于新手势，不能继承上次原生拖动的点击抑制。
  suppressClick = false;
  releaseFilePointer();
  if (
    event.button !== 0 ||
    row.name === ".." ||
    (row.kind !== "file" && row.kind !== "directory") ||
    stale.value ||
    busy.value ||
    (event.target as HTMLElement).closest(".n-checkbox")
  )
    return;
  const chosen = checkedNames.value.includes(row.name)
    ? selectedEntries.value
    : [row];
  if (chosen.some((item) => item.kind !== "file" && item.kind !== "directory"))
    return;
  // 捕获指针，移出当前行后仍能跨过阈值并发起原生拖动。
  const element = event.currentTarget as HTMLElement;
  element.setPointerCapture(event.pointerId);
  filePointer = { element, id: event.pointerId };
  dragStart = {
    x: event.clientX,
    y: event.clientY,
    names: chosen.map((item) => item.name),
    directories: chosen
      .filter((item) => item.kind === "directory")
      .map((item) => item.name),
  };
}
function moveFileDrag(event: PointerEvent) {
  if (!dragStart || !(event.buttons & 1) || stale.value || busy.value) {
    releaseFilePointer();
    return;
  }
  if (Math.hypot(event.clientX - dragStart.x, event.clientY - dragStart.y) < 6)
    return;
  event.preventDefault();
  const start = dragStart;
  releaseFilePointer();
  suppressClick = true;
  // 文件承诺由原生拖动会话提供，不构造浏览器下载链接或伪造文件路径。
  enqueue({
    sessionId: props.tab.sessionId!,
    path: directory.value,
    upload: false,
    name: start.names.join("、"),
    dragNames: start.names,
    dragDirectories: start.directories,
  });
}
function rowProps(row: FileEntry) {
  return {
    onPointerdown: (event: PointerEvent) => startFileDrag(event, row),
    onPointermove: moveFileDrag,
    onPointerup: releaseFilePointer,
    onPointercancel: releaseFilePointer,
    onLostpointercapture: () => {
      // 旧捕获的迟到事件不能清除新手势的指针状态。
      if (filePointer && !filePointer.element.hasPointerCapture(filePointer.id))
        releaseFilePointer();
    },
    onClick: (event: MouseEvent) => {
      if (suppressClick) {
        suppressClick = false;
        return;
      }
      if (
        row.name === ".." ||
        stale.value ||
        busy.value ||
        (event.target as HTMLElement).closest(".n-checkbox")
      )
        return;
      checkedNames.value = [row.name];
    },
    onDblclick: () => {
      if (
        isDirectory(row) &&
        !stale.value &&
        !busy.value &&
        !(row.name === ".." && atRoot.value)
      )
        void load(`${directory.value.replace(/\/$/, "")}/${row.name}`);
    },
    style: {
      cursor: row.name === ".." && atRoot.value ? "not-allowed" : "pointer",
      opacity: row.name === ".." && atRoot.value ? 0.4 : 1,
    },
    "aria-disabled": row.name === ".." && atRoot.value,
    "aria-selected": checkedNames.value.includes(row.name),
  };
}
watch(
  () => [props.tab.sessionId, props.tab.state] as const,
  () => {
    generation++;
    loading.value = false;
    dropTarget.value = undefined;
    for (const task of tasks.value)
      if (task.status === "queued") cancelQueued(task);
    cancelActive();
    if (connected.value && !transferring.value) void load(".");
  },
  { immediate: true },
);
onMounted(async () => {
  const listeners = await Promise.allSettled([
    desktop.onSftpLocalHover((position) => {
      dropTarget.value = targetAt(position);
    }),
    desktop.onSftpLocalDrop((drop) => {
      void receiveDrop(drop);
    }),
    desktop.onSftpLocalLeave(() => {
      dropTarget.value = undefined;
    }),
  ]);
  for (const result of listeners) {
    if (result.status === "fulfilled") {
      if (disposed) result.value();
      else eventUnlisteners.push(result.value);
    } else error.value = errorMessage(result.reason);
  }
});
onBeforeUnmount(() => {
  releaseFilePointer();
  suppressClick = false;
  for (const unlisten of eventUnlisteners) unlisten();
  disposed = true;
  generation++;
  cancelActive();
});
</script>
<template>
  <aside
    ref="panel"
    :style="{ flexBasis: panelWidth + 'px' }"
    class="sftp-panel"
    :class="{ 'sftp-drop-target': dropTarget !== undefined }"
    aria-label="此终端的 SFTP 文件浏览器"
    @keydown.stop
  >
    <div
      class="sftp-resizer"
      role="separator"
      tabindex="0"
      aria-label="调整 SFTP 侧栏宽度"
      aria-orientation="vertical"
      :aria-valuenow="panelWidth"
      @pointerdown="startResize"
      @pointermove="moveResize"
      @pointerup="stopResize"
      @pointercancel="stopResize"
      @lostpointercapture="resizeStart = undefined"
      @keydown="resizeKey"
    />
    <NFlex justify="space-between" align="center">
      <NText strong>SFTP · {{ tab.title }}</NText>
      <NButton
        size="tiny"
        @click="tab.sftpOpen = false"
        :title="visibleTransfer ? '隐藏面板，传输继续' : '隐藏 SFTP 侧栏'"
        >隐藏</NButton
      >
    </NFlex>
    <NAlert v-if="stale" type="warning" :show-icon="false">{{
      connected ? "列表已过期，请刷新" : "SSH 未连接，列表已过期"
    }}</NAlert>
    <NAlert
      v-if="error"
      type="error"
      :show-icon="false"
      closable
      @close="error = ''"
      >{{ error }}</NAlert
    >
    <NAlert
      v-if="dropTarget !== undefined"
      class="sftp-drop-indicator"
      type="info"
      :show-icon="false"
      >松开上传到 {{ dropTarget }}</NAlert
    >
    <NText v-if="notice" type="success">{{ notice }}</NText>
    <NFlex :wrap="false">
      <NForm
        ref="pathForm"
        :model="{ path: pathInput }"
        :rules="pathRules"
        @submit.prevent="navigatePath"
        class="path-form"
        ><NFormItem label="远端路径" path="path" :show-label="false">
          <NInput
            v-model:value="pathInput"
            size="small"
            placeholder="远端路径"
            :disabled="busy || !connected"
            aria-label="远端路径" /></NFormItem
      ></NForm>
      <NButton size="small" :disabled="busy || !connected" @click="navigatePath"
        >前往</NButton
      >
    </NFlex>
    <NFlex>
      <NButton size="tiny" :disabled="busy || !connected" @click="load('.')"
        >主目录</NButton
      >
      <NButton
        size="tiny"
        :disabled="busy || stale || atRoot"
        @click="load(directory.replace(/\/$/, '') + '/..')"
        >上级</NButton
      >
      <NButton size="tiny" :disabled="busy || !connected" @click="load()"
        >刷新</NButton
      >
      <NCheckbox
        :checked="hidden"
        :disabled="settings.savingSftpHidden"
        size="small"
        @update:checked="updateHidden"
        >显示隐藏文件</NCheckbox
      >
    </NFlex>
    <NFlex>
      <NButton size="tiny" :disabled="loading || stale" @click="transfer(true)"
        >上传</NButton
      >
      <NButton
        size="tiny"
        :disabled="busy || stale || selected?.kind !== 'file'"
        @click="transfer(false)"
        >下载</NButton
      >
    </NFlex>
    <NFlex v-if="visibleTransfer" align="center" justify="space-between">
      <NText>{{
        progress
          ? `已传输 ${formatFileSize(progress.transferred)} / ${progress.total === null ? "?" : formatFileSize(progress.total)}`
          : "选择文件或准备传输…"
      }}</NText>
      <NButton size="tiny" @click="cancel">取消传输</NButton>
    </NFlex>
    <NScrollbar
      v-if="visibleTasks.length"
      class="sftp-transfer-queue"
      style="max-height: 120px"
      aria-label="传输队列"
    >
      <NFlex
        v-for="task in visibleTasks"
        :key="task.id"
        align="center"
        justify="space-between"
        :wrap="false"
      >
        <NText
          :type="task.status === 'failed' ? 'error' : undefined"
          class="sftp-task-label"
          :title="task.message || task.path"
        >
          {{ task.upload ? "上传" : "下载" }} ·
          {{ task.progress?.currentName || task.name || task.path }} ·
          {{
            task.status === "queued"
              ? "等待中"
              : task.status === "running"
                ? `传输中${task.progress?.totalFiles ? ` ${task.progress.completedFiles || 0}/${task.progress.totalFiles}` : ""}`
                : task.message
          }}
        </NText>
        <NButton
          v-if="task.status === 'queued'"
          size="tiny"
          @click="cancelQueued(task)"
          >移除</NButton
        >
      </NFlex>
    </NScrollbar>
    <NDataTable
      class="sftp-table"
      size="small"
      flex-height
      virtual-scroll
      remote
      v-model:checked-row-keys="checkedNames"
      @update:sorter="updateSort"
      :columns="columns"
      :data="rows"
      :row-key="(row) => row.name"
      :row-props="rowProps"
      :row-class-name="
        (row) => (checkedNames.includes(row.name) ? 'sftp-selected' : '')
      "
      :loading="loading"
      :scroll-x="356"
      :single-line="false"
    />
    <NFlex justify="space-between" align="center">
      <NText depth="3"
        >{{ rows.length - 1 }} 项{{ hasMore ? "（尚未全部加载）" : "" }}</NText
      >
      <NButton
        v-if="hasMore"
        size="tiny"
        :disabled="busy || stale || entries.length >= 10000"
        @click="load(directory, true)"
        >加载更多（100 项）</NButton
      >
    </NFlex>
    <NText v-if="entries.length >= 10000 && hasMore" type="warning"
      >已达当前视图条目上限，请按路径进入子目录。</NText
    >
    <NText v-if="selectedEntries.length" depth="3" class="sftp-selection"
      >已选择 {{ selectedEntries.length }} 项{{
        selected ? `：${selected.name}` : ""
      }}</NText
    >
  </aside>
</template>
<style scoped>
.path-form {
  flex: 1;
  min-width: 0;
}
.sftp-panel {
  position: relative;
  max-width: calc(100% - 100px);
  display: flex;
  flex: 0 0 clamp(300px, 38%, 560px);
  flex-direction: column;
  min-width: 240px;
  min-height: 0;
  gap: 10px;
  padding: 12px;
  border-left: 1px solid var(--border);
  background: var(--surface);
  overflow: hidden;
}
.sftp-drop-indicator {
  position: absolute;
  top: 40px;
  left: 12px;
  right: 12px;
  z-index: 3;
  pointer-events: none;
}
.sftp-drop-target {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.sftp-transfer-queue {
  max-height: 120px;
  flex: 0 1 auto;
  flex-shrink: 0;
}
.sftp-task-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sftp-resizer {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  width: 6px;
  cursor: col-resize;
  touch-action: none;
  z-index: 2;
}
.sftp-table {
  flex: 1;
  min-height: 80px;
}
.sftp-selection {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
:deep(.sftp-entry-name) {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
:deep(.sftp-entry-name .n-icon) {
  flex-shrink: 0;
}
:deep(.sftp-entry-label) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
:deep(.sftp-selected td) {
  background: var(--n-td-color-hover) !important;
}
</style>

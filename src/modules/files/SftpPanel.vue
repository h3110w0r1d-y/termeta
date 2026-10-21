<script setup lang="ts">
import { t } from "@/i18n";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NCheckbox,
  NFlex,
  NText,
  NConfigProvider,
} from "naive-ui";
import { desktop, errorMessage } from "@/services/desktop/client";
import type {
  FileEntry,
  SftpLocalDrop,
  SftpDropPosition,
} from "@/services/desktop/types";
import { useWorkbenchStore } from "../workbench/store";
import type { TerminalItem } from "../workbench/store";
import { confirmAction } from "@/components/ui/confirmation";
import { isDirectory } from "./fileEntries";
import { useSftpTransferQueue } from "./useSftpTransferQueue";
import SftpToolbar from "./SftpToolbar.vue";
import SftpFileTable from "./SftpFileTable.vue";
import SftpFileActions from "./SftpFileActions.vue";
import SftpTransferFooter from "./SftpTransferFooter.vue";
import type { SftpTransferActivity } from "./transferActivity";
import { IconUpload, IconDownload, IconX } from "@tabler/icons-vue";
import { useSettings } from "../settings/store";
import { schemeForTerminal } from "../terminal/registry";
import { useTerminalSurface } from "../terminal/useTerminalSurface";
import { sftpWorkingDirectory } from "../terminal/workingDirectory";
import { useSftpDirectoryFollow } from "./useSftpDirectoryFollow";
import { useSftpFileDrag } from "./useSftpFileDrag";
import { sftpPathArguments } from "./sftpPathArguments";
const props = defineProps<{
  terminal: TerminalItem;
  terminalHost?: HTMLElement;
}>();
const emit = defineEmits<{
  pastePaths: [sessionId: string, text: string];
}>();
const {
  theme,
  themeOverrides,
  style: surfaceStyle,
} = useTerminalSurface(() => schemeForTerminal(props.terminal).colors);
const panel = ref<HTMLElement>();
const workbench = useWorkbenchStore();
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
const entries = ref<FileEntry[]>([]);
const checkedNames = ref<string[]>([]);
const selectedEntries = computed(() => {
  const names = new Set(checkedNames.value);
  return entries.value.filter((entry) => names.has(entry.name));
});
const selected = computed(() =>
  selectedEntries.value.length === 1 ? selectedEntries.value[0] : undefined,
);
const mutationBusy = ref(false);
const fileActions = ref<InstanceType<typeof SftpFileActions>>();
const atRoot = computed(() => directory.value === "/");
const settings = useSettings();
const hidden = computed(() => settings.sftpShowHidden);
const followDirectory = ref(false);
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
const {
  tasks,
  visibleTasks,
  visibleTransfer,
  transferring,
  enqueue,
  cancelQueued,
  cancel,
  cancelActive,
} = useSftpTransferQueue({
  terminal: () => props.terminal,
  busy: () => busy.value,
  disposed: () => disposed,
  connected: () => connected.value,
  sameSession,
  refresh: (path) => load(path),
  error,
});

const dropTarget = ref<string>();
const consumedDrops = new Set<string>();
const eventUnlisteners: (() => void)[] = [];
const pendingDrop = ref(false);
const listSession = ref<string>();
const connected = computed(
  () => props.terminal.state === "connected" && !!props.terminal.sessionId,
);
const stale = computed(
  () => !connected.value || listSession.value !== props.terminal.sessionId,
);
const busy = computed(
  () => loading.value || transferring.value || mutationBusy.value,
);
const fileDrag = useSftpFileDrag({
  terminalHost: () => props.terminalHost,
  valid: (selection) =>
    sameSession(selection.sessionId) &&
    !busy.value &&
    !pendingDrop.value &&
    !fileActions.value?.interacting &&
    !!props.terminal.sftpOpen &&
    directory.value === selection.path &&
    workbench.activeTerminal?.terminalId === props.terminal.terminalId,
  paste: (selection) => {
    const text = sftpPathArguments(selection.path, selection.names);
    if (text === undefined) error.value = t("files.drag.invalidPaths");
    else emit("pastePaths", selection.sessionId, text);
  },
  native: (selection) =>
    enqueue({
      sessionId: selection.sessionId,
      path: selection.path,
      upload: false,
      name: selection.names.join("、"),
      dragNames: selection.names,
      dragDirectories: selection.directories,
    }),
});
const dragFeedback = fileDrag.feedback;
const dragPreviewStyle = fileDrag.previewStyle;
const activity = computed<SftpTransferActivity[]>(() =>
  visibleTasks.value.map((task) => ({
    id: task.id,
    label: `${task.upload ? t("files.actions.upload") : t("files.actions.download")} · ${task.name || task.path}`,
    path: task.path,
    status: task.status,
    progress: task.progress,
    message: task.message,
    messageKey: task.messageKey,
    cancelling: task.cancelRequested,
  })),
);
function cancelTask(id: string) {
  const task = tasks.value.find((task) => task.id === id);
  if (task?.status === "queued") cancelQueued(task);
  else if (task?.status === "running") void cancel();
}
function clearFinished() {
  tasks.value = tasks.value.filter(
    (task) => task.status === "queued" || task.status === "running",
  );
}
let generation = 0;
let disposed = false;
async function load(path = directory.value, more = false) {
  const sessionId = props.terminal.sessionId;
  if (!connected.value || !sessionId || busy.value || disposed) return;
  fileDrag.cancel();
  const current = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const page = await desktop.sftpList(sessionId, path, more);
    if (
      disposed ||
      current !== generation ||
      sessionId !== props.terminal.sessionId
    )
      return;
    directory.value = page.path;
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
    sessionId === props.terminal.sessionId &&
    listSession.value === sessionId
  );
}
async function transfer(upload: boolean) {
  if (
    stale.value ||
    loading.value ||
    mutationBusy.value ||
    (!upload && selected.value?.kind !== "file")
  )
    return;
  enqueue({
    sessionId: props.terminal.sessionId!,
    path: directory.value,
    upload,
    name: upload ? null : selected.value!.name,
  });
}
function targetAt(position: SftpDropPosition): string | undefined {
  if (
    !props.terminal.sftpOpen ||
    workbench.activeTerminal?.terminalId !== props.terminal.terminalId ||
    stale.value ||
    mutationBusy.value ||
    loading.value ||
    fileDrag.dragging.value ||
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
  if (consumedDrops.has(drop.dropId) || pendingDrop.value) return;
  consumedDrops.add(drop.dropId);
  if (consumedDrops.size > 256)
    consumedDrops.delete(consumedDrops.values().next().value!);
  pendingDrop.value = true;
  const sessionId = props.terminal.sessionId!;
  try {
    if (
      await confirmAction(t("files.upload.confirmDrop", { target: target }))
    ) {
      if (sameSession(sessionId) && !mutationBusy.value && !loading.value)
        enqueue({
          sessionId,
          path: target,
          upload: true,
          name: null,
          dropId: drop.dropId,
        });
    }
  } finally {
    pendingDrop.value = false;
  }
}
function startFileDrag(event: PointerEvent, chosen: FileEntry[]) {
  if (!props.terminal.sessionId) return;
  fileDrag.start(event, {
    sessionId: props.terminal.sessionId,
    path: directory.value,
    names: chosen.map((item) => item.name),
    directories: chosen
      .filter((item) => item.kind === "directory")
      .map((item) => item.name),
    nativeAllowed: chosen.every(
      (item) => item.kind === "file" || item.kind === "directory",
    ),
  });
}
function navigateEntry(row: FileEntry) {
  if (!busy.value && !stale.value && isDirectory(row))
    void load(`${directory.value.replace(/\/$/, "")}/${row.name}`);
}
watch(
  () => [props.terminal.sessionId, props.terminal.state] as const,
  () => {
    fileDrag.cancel();
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
watch(
  () =>
    [props.terminal.sftpOpen, workbench.activeTerminal?.terminalId] as const,
  ([open, terminalId]) => {
    if (!open || terminalId !== props.terminal.terminalId) fileDrag.cancel();
  },
);
useSftpDirectoryFollow({
  enabled: () => followDirectory.value,
  sessionId: () => (connected.value ? props.terminal.sessionId : undefined),
  path: () => props.terminal.workingDirectory,
  available: () =>
    !!props.terminal.sftpOpen &&
    connected.value &&
    !busy.value &&
    !fileActions.value?.interacting &&
    !pendingDrop.value &&
    !fileDrag.dragging.value,
  navigate: async (path) => {
    const target = sftpWorkingDirectory(path);
    if (target !== directory.value || stale.value) await load(target);
  },
});
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
  fileDrag.cancel();
  for (const unlisten of eventUnlisteners) unlisten();
  disposed = true;
  generation++;
  cancelActive();
});
</script>
<template>
  <aside
    ref="panel"
    :style="[surfaceStyle, { flexBasis: panelWidth + 'px' }]"
    class="sftp-panel"
    :class="{ 'sftp-drop-target': dropTarget !== undefined }"
    :aria-label="t('files.sidebar.label')"
    @keydown.stop
  >
    <NConfigProvider abstract :theme="theme" :theme-overrides="themeOverrides">
      <div
        class="sftp-resizer"
        role="separator"
        tabindex="0"
        :aria-label="t('files.sidebar.resize')"
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
        <NText strong>SFTP · {{ terminal.title }}</NText>
        <NButton
          size="tiny"
          quaternary
          :aria-label="t('files.sidebar.hide')"
          @click="terminal.sftpOpen = false"
          :title="
            visibleTransfer
              ? t('files.transfer.hidePanel')
              : t('files.sidebar.hide')
          "
        >
          <template #icon><IconX :size="16" aria-hidden="true" /></template>
        </NButton>
      </NFlex>
      <NAlert v-if="stale" type="warning" :show-icon="false">
        {{
          connected
            ? t("files.errors.staleList")
            : t("files.errors.sshDisconnected")
        }}
      </NAlert>
      <NAlert
        v-if="error"
        type="error"
        :show-icon="false"
        closable
        @close="error = ''"
      >
        {{ error }}
      </NAlert>
      <NAlert
        v-if="dropTarget !== undefined"
        class="sftp-drop-indicator"
        type="info"
        :show-icon="false"
      >
        {{ t("files.upload.dropHint", { path: dropTarget }) }}
      </NAlert>
      <SftpToolbar
        :path="directory"
        :disabled="busy || !connected"
        :show-hidden="hidden"
        :saving-hidden="settings.savingSftpHidden"
        @navigate="load"
        @refresh="load()"
        @hidden="updateHidden"
      >
        <template #preferences>
          <NCheckbox
            v-model:checked="followDirectory"
            size="small"
            :title="t('files.follow.description')"
          >
            {{ t("files.follow.label") }}
          </NCheckbox>
          <NText
            v-if="followDirectory && !terminal.workingDirectory"
            depth="3"
            class="follow-hint"
          >
            {{ t("files.follow.waiting") }}
          </NText>
        </template>
        <NButton
          size="small"
          :disabled="loading || mutationBusy || stale"
          @click="transfer(true)"
        >
          <template #icon>
            <IconUpload :size="16" aria-hidden="true" />
          </template>
          {{ t("files.actions.upload") }}
        </NButton>
        <NButton
          size="small"
          :disabled="busy || stale || selected?.kind !== 'file'"
          @click="transfer(false)"
        >
          <template #icon>
            <IconDownload :size="16" aria-hidden="true" />
          </template>
          {{ t("files.actions.download") }}
        </NButton>
      </SftpToolbar>
      <SftpFileTable
        v-model:selected="checkedNames"
        :entries="entries"
        :show-hidden="hidden"
        :disabled="busy || stale"
        :loading="loading"
        :at-root="atRoot"
        @navigate="navigateEntry"
        @drag="startFileDrag"
        @contextmenu="(event, row) => fileActions?.open(event, row)"
      />
      <div class="panel-status">
        <NText depth="3">
          {{
            t("files.selection.loaded", {
              loaded: entries.filter(
                (entry) => hidden || !entry.name.startsWith("."),
              ).length,
              selected: checkedNames.length,
            })
          }}
        </NText>
        <NButton
          v-if="hasMore"
          size="tiny"
          :disabled="busy || stale || entries.length >= 10000"
          @click="load(directory, true)"
        >
          {{ t("files.actions.loadMorePage") }}
        </NButton>
      </div>
      <NText v-if="entries.length >= 10000 && hasMore" type="warning">
        {{ t("files.errors.tooManyEntries") }}
      </NText>
      <SftpTransferFooter
        :tasks="activity"
        @cancel="cancelTask"
        @clear-finished="clearFinished"
      />
      <SftpFileActions
        ref="fileActions"
        :session-id="terminal.sessionId"
        :path="directory"
        :entries="selectedEntries"
        :disabled="loading || transferring || stale"
        @busy="mutationBusy = $event"
        @changed="load()"
      />
      <Teleport to="body">
        <div
          v-if="dragFeedback?.active"
          class="sftp-drag-preview"
          :style="[surfaceStyle, dragPreviewStyle]"
        >
          {{
            t(
              dragFeedback.overTerminal
                ? "files.drag.pastePaths"
                : "files.drag.hint",
              { count: dragFeedback.count },
            )
          }}
        </div>
      </Teleport>
    </NConfigProvider>
  </aside>
</template>
<style scoped>
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
.panel-status {
  display: flex;
  justify-content: space-between;
  align-items: center;
  min-height: 26px;
  gap: 8px;
  font-size: 12px;
}
.follow-hint {
  flex-basis: 100%;
  font-size: 11px;
}
.sftp-drag-preview {
  position: fixed;
  z-index: 3000;
  max-width: 270px;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text);
  box-shadow: 0 4px 16px #0004;
  pointer-events: none;
  font-size: 12px;
}
</style>

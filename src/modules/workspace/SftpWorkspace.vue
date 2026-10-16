<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
} from "vue";
import { NAlert, NButton, NFlex, NTag, NText } from "naive-ui";
import { desktop, errorMessage } from "../../services/desktop/client";
import type {
  SessionEvent,
  SessionSnapshot,
  TransferProgress,
} from "../../services/desktop/types";
import { useWorkspace } from "./store";
import SftpEndpointPane from "../files/SftpEndpointPane.vue";
import type { SftpEndpoint, SftpSelection } from "../files/workspace";
import { formatFileSize } from "../files/format";

const workspace = useWorkspace();
const endpoints = reactive<SftpEndpoint[]>(
  [0, 1].map(() => ({
    profileId: null,
    state: "closed",
    error: "",
    opening: false,
  })),
);
const left = ref<InstanceType<typeof SftpEndpointPane>>();
const right = ref<InstanceType<typeof SftpEndpointPane>>();
const panes = [left, right];
const root = ref<HTMLElement>();
const copying = ref(false);
const progress = ref<TransferProgress>();
const message = ref("");
const error = ref("");
let disposed = false;
const generations = [0, 0];
const revisions = [-1, -1];
const ended = [false, false];
let cancelRequested = false;
let currentCopy: { sessionId: string; id: string } | undefined;
const ready = (side: number) =>
  endpoints[side]?.state === "connected" && !!endpoints[side]?.sessionId;
function update(side: number, snapshot: SessionSnapshot) {
  if (ended[side] || snapshot.stateRevision < revisions[side]) return;
  revisions[side] = snapshot.stateRevision;
  workspace.fileSessionTitles[snapshot.sessionId] = snapshot.title;
  endpoints[side]!.sessionId = snapshot.sessionId;
  endpoints[side]!.state = snapshot.state;
}
function control(side: number, generation: number, event: SessionEvent) {
  if (disposed || generations[side] !== generation) return;
  const endpoint = endpoints[side]!;
  endpoint.sessionId = event.sessionId;
  if (event.type === "stateChanged") update(side, event.snapshot);
  else if (
    event.type === "authChallenge" ||
    event.type === "hostKeyChallenge"
  ) {
    workspace.dismiss(event.sessionId);
    workspace.challenges.push(event);
  } else if (event.type === "ended") {
    ended[side] = true;
    releaseDrag();
    endpoint.state = event.finalState;
    endpoint.error = event.error?.message ?? "";
    workspace.dismiss(event.sessionId);
    delete workspace.fileSessionTitles[event.sessionId];
    if (currentCopy)
      void desktop
        .sftpCancel(currentCopy.sessionId, currentCopy.id)
        .catch(() => {});
  }
}
async function connect(side: number, profileId: string) {
  const endpoint = endpoints[side]!;
  if (endpoint.sessionId || endpoint.opening || copying.value) return;
  const generation = ++generations[side]!;
  endpoint.profileId = profileId;
  endpoint.opening = true;
  endpoint.error = "";
  ended[side] = false;
  revisions[side] = -1;
  try {
    const snapshot = await desktop.sftpConnect(
      profileId,
      crypto.randomUUID(),
      (event) => control(side, generation, event),
    );
    if (disposed || generations[side] !== generation) {
      await desktop.close(snapshot.sessionId);
      return;
    }
    update(side, snapshot);
  } catch (cause) {
    endpoint.error = errorMessage(cause);
  } finally {
    endpoint.opening = false;
  }
}
async function disconnect(side: number) {
  const endpoint = endpoints[side]!;
  if (!endpoint.sessionId || copying.value) return;
  endpoint.opening = true;
  try {
    await desktop.close(endpoint.sessionId);
    workspace.dismiss(endpoint.sessionId);
    delete workspace.fileSessionTitles[endpoint.sessionId];
    generations[side]!++;
    endpoint.sessionId = undefined;
    endpoint.state = "closed";
    endpoint.error = "";
  } catch (cause) {
    endpoint.error = errorMessage(cause);
  } finally {
    endpoint.opening = false;
  }
}
async function copy(side: number, selection: SftpSelection) {
  const target = 1 - side;
  const sourceSessionId = endpoints[side]!.sessionId;
  const targetSessionId = endpoints[target]!.sessionId;
  const targetPath = panes[target]!.value?.path;
  if (
    copying.value ||
    !ready(side) ||
    !ready(target) ||
    !sourceSessionId ||
    selection.sessionId !== sourceSessionId ||
    !panes[target]!.value?.available ||
    !targetSessionId ||
    !targetPath ||
    targetPath === "."
  )
    return;
  const transferId = crypto.randomUUID();
  cancelRequested = false;
  copying.value = workspace.fileTransferring = true;
  currentCopy = { sessionId: sourceSessionId, id: transferId };
  progress.value = undefined;
  message.value = "";
  error.value = "";
  try {
    await desktop.sftpCopy(
      {
        sourceSessionId,
        targetSessionId,
        sourcePath: selection.path,
        targetPath,
        names: selection.names,
        transferId,
      },
      (value) => {
        if (currentCopy?.id === transferId) {
          progress.value = value;
          // 取消可能早于后端登记；首次进度确认登记后再补发一次。
          if (cancelRequested) {
            cancelRequested = false;
            void desktop
              .sftpCancel(sourceSessionId, transferId)
              .catch(() => {});
          }
        }
      },
    );
    message.value = "对拷完成";
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    currentCopy = undefined;
    copying.value = workspace.fileTransferring = false;
    // 等待busy属性同步到子组件，再刷新两端目录。
    await nextTick();
    if (!disposed)
      await Promise.all(panes.map((pane) => pane.value?.refresh()));
  }
}
async function cancelCopy() {
  if (!currentCopy) return;
  cancelRequested = true;
  try {
    await desktop.sftpCancel(currentCopy.sessionId, currentCopy.id);
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}
// 栏间拖动只使用Pointer Events；不设置draggable或文件DataTransfer，避免原生拖放接管。
interface Gesture extends SftpSelection {
  side: number;
  pointerId: number;
  element: HTMLElement;
  x: number;
  y: number;
  moved: boolean;
}
let gesture: Gesture | undefined;
const drag = ref<{ x: number; y: number; count: number; target?: number }>();
function targetAt(x: number, y: number) {
  const node = document
    .elementFromPoint(x, y)
    ?.closest<HTMLElement>("[data-sftp-side]");
  if (!node || !root.value?.contains(node)) return;
  const side = Number(node.dataset.sftpSide);
  if (ready(side) && panes[side]!.value?.available && side !== gesture?.side)
    return side;
}
function startDrag(
  side: number,
  event: PointerEvent,
  selection: SftpSelection,
) {
  releaseDrag();
  if (copying.value || workspace.activeId !== "sftp") return;
  const element = event.currentTarget as HTMLElement;
  element.setPointerCapture(event.pointerId);
  gesture = {
    ...selection,
    side,
    pointerId: event.pointerId,
    element,
    x: event.clientX,
    y: event.clientY,
    moved: false,
  };
}
function moveDrag(event: PointerEvent) {
  if (!gesture || event.pointerId !== gesture.pointerId) return;
  if (!(event.buttons & 1)) {
    releaseDrag();
    return;
  }
  if (
    !gesture.moved &&
    Math.hypot(event.clientX - gesture.x, event.clientY - gesture.y) < 6
  )
    return;
  gesture.moved = true;
  event.preventDefault();
  drag.value = {
    x: event.clientX,
    y: event.clientY,
    count: gesture.names.length,
    target: targetAt(event.clientX, event.clientY),
  };
}
function endDrag(event: PointerEvent) {
  if (!gesture || event.pointerId !== gesture.pointerId) return;
  const current = gesture;
  const target = current.moved
    ? targetAt(event.clientX, event.clientY)
    : undefined;
  releaseDrag();
  if (target !== undefined) void copy(current.side, current);
}
function releaseDrag() {
  const current = gesture;
  gesture = undefined;
  drag.value = undefined;
  if (current?.element.hasPointerCapture(current.pointerId))
    current.element.releasePointerCapture(current.pointerId);
}
function keydown(event: KeyboardEvent) {
  if (event.key === "Escape") releaseDrag();
}
watch(() => workspace.activeId, releaseDrag);
const detail = computed(() => {
  const value = progress.value;
  if (!value) return "准备对拷…";
  return `${value.currentName ?? "正在传输"} · ${value.completedFiles ?? 0}/${value.totalFiles ?? "?"} 个文件 · ${formatFileSize(value.transferred)}${value.total ? ` / ${formatFileSize(value.total)}` : ""}`;
});
onMounted(() => {
  window.addEventListener("pointermove", moveDrag, { passive: false });
  window.addEventListener("pointerup", endDrag);
  window.addEventListener("pointercancel", releaseDrag);
  window.addEventListener("blur", releaseDrag);
  window.addEventListener("keydown", keydown);
});
onBeforeUnmount(() => {
  disposed = true;
  releaseDrag();
  window.removeEventListener("pointermove", moveDrag);
  window.removeEventListener("pointerup", endDrag);
  window.removeEventListener("pointercancel", releaseDrag);
  window.removeEventListener("blur", releaseDrag);
  window.removeEventListener("keydown", keydown);
  for (const endpoint of endpoints)
    if (endpoint.sessionId) {
      workspace.dismiss(endpoint.sessionId);
      delete workspace.fileSessionTitles[endpoint.sessionId];
      void desktop.close(endpoint.sessionId).catch(() => {});
    }
});
</script>

<template>
  <section
    id="page-sftp"
    ref="root"
    aria-labelledby="workspace-tab-sftp"
    class="sftp-workspace"
    role="tabpanel"
    aria-label="双栏 SFTP 文件工作区"
  >
    <NAlert v-if="error" type="error" closable @close="error = ''">
      {{ error }}
    </NAlert>
    <NAlert v-else-if="message" type="success" closable @close="message = ''">
      {{ message }}
    </NAlert>
    <NFlex v-if="copying" align="center" :wrap="false">
      <NText class="copy-progress">{{ detail }}</NText>
      <NButton size="small" @click="cancelCopy">取消对拷</NButton>
    </NFlex>
    <div class="endpoint-grid">
      <div
        v-for="side in [0, 1]"
        :key="side"
        :data-sftp-side="side"
        class="endpoint-container"
        :class="{ 'copy-target': drag?.target === side }"
      >
        <SftpEndpointPane
          :ref="
            (value) => {
              if (side === 0)
                left = value as
                  InstanceType<typeof SftpEndpointPane> | undefined;
              else
                right = value as
                  InstanceType<typeof SftpEndpointPane> | undefined;
            }
          "
          :endpoint="endpoints[side]!"
          :busy="copying"
          :can-copy="ready(1 - side)"
          @connect="(profile) => connect(side, profile)"
          @disconnect="disconnect(side)"
          @drag="(event, selection) => startDrag(side, event, selection)"
          @copy="(selection) => copy(side, selection)"
        />
      </div>
    </div>
    <Teleport to="body">
      <div
        v-if="drag"
        class="copy-drag-label"
        :style="{ left: `${drag.x + 14}px`, top: `${drag.y + 14}px` }"
      >
        <NTag :type="drag.target !== undefined ? 'success' : 'default'">
          复制 {{ drag.count }} 项
          {{ drag.target !== undefined ? "到对侧当前目录" : " · 拖到另一栏" }}
        </NTag>
      </div>
    </Teleport>
  </section>
</template>

<style scoped>
.sftp-workspace {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 34px;
}

.sftp-workspace {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  overflow: hidden;
  min-width: 0;
}
.endpoint-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 12px;
  flex: 1;
  min-height: 0;
}
.endpoint-container {
  min-height: 0;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}
.copy-target {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.copy-progress {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
.copy-drag-label {
  position: fixed;
  z-index: 3000;
  pointer-events: none;
}
</style>

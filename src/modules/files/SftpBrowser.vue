<script setup lang="ts">
import { t } from "@/i18n";
import {
  nextTick,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
} from "vue";
import { NAlert, NTag } from "naive-ui";
import { desktop, errorMessage } from "@/services/desktop/client";
import { useSessions } from "../sessions/store";
import { createSessionControl, isSessionTerminal } from "../sessions/control";
import type { SessionEvent, SessionSnapshot } from "@/services/desktop/types";
import { useWorkbenchStore } from "../workbench/store";
import SftpEndpointPane from "./SftpEndpointPane.vue";
import type { SftpEndpoint, SftpSelection } from "./types";
import SftpTransferFooter from "./SftpTransferFooter.vue";
import type { SftpTransferActivity } from "./transferActivity";

const workbench = useWorkbenchStore();
const sessions = useSessions();
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
const tasks = ref<SftpTransferActivity[]>([]);
const error = ref("");
let disposed = false;
const generations = [0, 0];
const controls = [createSessionControl(""), createSessionControl("")];
let cancelRequested = false;
let currentCopy: { sessionId: string; id: string } | undefined;
const ready = (side: number) =>
  endpoints[side]?.state === "connected" && !!endpoints[side]?.sessionId;
function update(side: number, snapshot: SessionSnapshot) {
  if (!sessions.snapshot(controls[side]!, snapshot, "file")) return;
  endpoints[side]!.sessionId = snapshot.sessionId;
  endpoints[side]!.state = snapshot.state;
}
function control(side: number, generation: number, event: SessionEvent) {
  if (disposed || generations[side] !== generation) return;
  const projection = controls[side]!;
  const title =
    event.type === "stateChanged"
      ? event.snapshot.title
      : (sessions.titles[event.sessionId] ?? t("files.browser.title"));
  if (!sessions.event(projection, event, "file", title)) return;
  const endpoint = endpoints[side]!;
  endpoint.sessionId = projection.sessionId;
  endpoint.state = projection.state;
  if (event.type === "ended") {
    releaseDrag();
    endpoint.error = event.error ? errorMessage(event.error) : "";
    if (currentCopy)
      void desktop
        .sftpCancel(currentCopy.sessionId, currentCopy.id)
        .catch(() => {});
  }
}
async function connect(side: number, profileId: string) {
  const endpoint = endpoints[side]!;
  if (
    (endpoint.sessionId && !isSessionTerminal(endpoint.state)) ||
    endpoint.opening ||
    copying.value
  )
    return;
  const generation = ++generations[side]!;
  endpoint.profileId = profileId;
  endpoint.opening = true;
  endpoint.error = "";
  const projection = (controls[side] = createSessionControl(
    crypto.randomUUID(),
  ));
  try {
    // 终态仍可能保留后端资源；先关闭旧会话，再从主机列表建立独立连接。
    if (endpoint.sessionId) {
      await desktop.close(endpoint.sessionId);
      sessions.release("file", endpoint.sessionId);
      endpoint.sessionId = undefined;
      endpoint.state = "closed";
      if (disposed || generations[side] !== generation) return;
    }
    const snapshot = await desktop.sftpConnect(
      profileId,
      projection.requestId,
      (event) => control(side, generation, event),
    );
    if (disposed || generations[side] !== generation) {
      await desktop.close(snapshot.sessionId);
      return;
    }
    update(side, snapshot);
  } catch (cause) {
    if (!projection.ended) endpoint.error = errorMessage(cause);
  } finally {
    endpoint.opening = false;
  }
}
async function disconnect(side: number) {
  const endpoint = endpoints[side]!;
  if (!endpoint.sessionId || copying.value) return;
  endpoint.opening = true;
  const sessionId = endpoint.sessionId;
  try {
    await desktop.close(sessionId);
    sessions.release("file", sessionId);
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
    !panes[side]!.value?.available ||
    panes[side]!.value?.path !== selection.path ||
    !panes[target]!.value?.available ||
    !targetSessionId ||
    !targetPath ||
    targetPath === "."
  )
    return;
  const transferId = crypto.randomUUID();
  const task = reactive<SftpTransferActivity>({
    id: transferId,
    label: selection.names.join(", "),
    path: `${selection.path} → ${targetPath}`,
    status: "running",
  });
  tasks.value = [...tasks.value.slice(-19), task];
  cancelRequested = false;
  copying.value = workbench.fileTransferring = true;
  currentCopy = { sessionId: sourceSessionId, id: transferId };
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
          task.progress = value;
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
    task.status = "done";
    task.messageKey = "files.copy.complete";
  } catch (cause) {
    const cancelled =
      typeof cause === "object" &&
      cause !== null &&
      "code" in cause &&
      cause.code === "CANCELLED";
    task.status = cancelled ? "cancelled" : "failed";
    if (cancelled) task.messageKey = "common.status.cancelled";
    else task.message = errorMessage(cause);
  } finally {
    task.cancelling = false;
    currentCopy = undefined;
    copying.value = workbench.fileTransferring = false;
    // 等待busy属性同步到子组件，再刷新两端目录。
    await nextTick();
    if (!disposed)
      await Promise.all(panes.map((pane) => pane.value?.refresh()));
  }
}
async function cancelCopy(id: string) {
  if (!currentCopy || currentCopy.id !== id) return;
  cancelRequested = true;
  const task = tasks.value.find((item) => item.id === id);
  if (task) task.cancelling = true;
  try {
    await desktop.sftpCancel(currentCopy.sessionId, currentCopy.id);
  } catch (cause) {
    if (task) task.cancelling = false;
    error.value = errorMessage(cause);
  }
}
function clearFinished() {
  tasks.value = tasks.value.filter(
    (task) => task.status === "running" || task.status === "queued",
  );
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
  if (copying.value || workbench.activeId !== "sftp") return;
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
watch(() => workbench.activeId, releaseDrag);
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
      sessions.release("file", endpoint.sessionId);
      void desktop.close(endpoint.sessionId).catch(() => {});
    }
});
</script>

<template>
  <section
    id="page-sftp"
    ref="root"
    aria-labelledby="app-tab-sftp"
    class="sftp-browser"
    role="tabpanel"
    :aria-label="t('files.browser.title')"
  >
    <NAlert v-if="error" type="error" closable @close="error = ''">
      {{ error }}
    </NAlert>
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
    <SftpTransferFooter
      :tasks="tasks"
      @cancel="cancelCopy"
      @clear-finished="clearFinished"
    />
    <Teleport to="body">
      <div
        v-if="drag"
        class="copy-drag-label"
        :style="{ left: `${drag.x + 14}px`, top: `${drag.y + 14}px` }"
      >
        <NTag :type="drag.target !== undefined ? 'success' : 'default'">
          {{
            drag.target !== undefined
              ? t("files.copy.confirm", { count: drag.count })
              : t("files.copy.dragHint", { count: drag.count })
          }}
        </NTag>
      </div>
    </Teleport>
  </section>
</template>

<style scoped>
.sftp-browser {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  overflow: hidden;
  min-width: 0;
  min-height: 0;
  flex: 1;
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
.copy-drag-label {
  position: fixed;
  z-index: 3000;
  pointer-events: none;
}
</style>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import TerminalPane from "../terminal/TerminalPane.vue";
import { layoutGeometry, type LayoutRect, type SplitDivider } from "./layout";
import { useWorkspace } from "./store";
const emit = defineEmits<{
  close: [terminalId: string];
  menu: [event: MouseEvent | KeyboardEvent, terminalId: string];
}>();
const workspace = useWorkspace();
const stage = ref<HTMLElement>();
const geometry = computed(() =>
  workspace.activeGroup
    ? layoutGeometry(workspace.activeGroup.root)
    : { panes: new Map<string, LayoutRect>(), dividers: [] as SplitDivider[] },
);
function rectStyle(rect?: LayoutRect) {
  return rect
    ? {
        left: rect.left + "%",
        top: rect.top + "%",
        width: rect.width + "%",
        height: rect.height + "%",
      }
    : undefined;
}
const preview = computed(() => {
  const drop = workspace.dropTarget;
  if (
    !workspace.drag ||
    drop?.kind !== "pane" ||
    !workspace.canSplit(workspace.drag.source, drop.terminalId)
  )
    return;
  const rect = geometry.value.panes.get(drop.terminalId);
  if (!rect) return;
  const half = { ...rect };
  if (drop.edge === "left" || drop.edge === "right") {
    half.width /= 2;
    if (drop.edge === "right") half.left += half.width;
  } else {
    half.height /= 2;
    if (drop.edge === "bottom") half.top += half.height;
  }
  return half;
});
const resizing = ref<{
  id: string;
  pointerId: number;
  axis: SplitDivider["axis"];
  start: number;
  length: number;
  element: HTMLElement;
}>();
function startResize(event: PointerEvent, divider: SplitDivider) {
  if (event.button !== 0 || !event.isPrimary || workspace.drag || !stage.value)
    return;
  event.preventDefault();
  event.stopPropagation();
  const rect = stage.value.getBoundingClientRect();
  const horizontal = divider.axis === "horizontal";
  const element = event.currentTarget as HTMLElement;
  element.setPointerCapture(event.pointerId);
  resizing.value = {
    id: divider.id,
    pointerId: event.pointerId,
    axis: divider.axis,
    element,
    start: horizontal
      ? rect.left + (rect.width * divider.bounds.left) / 100
      : rect.top + (rect.height * divider.bounds.top) / 100,
    length: horizontal
      ? (rect.width * divider.bounds.width) / 100
      : (rect.height * divider.bounds.height) / 100,
  };
}
function moveResize(event: PointerEvent) {
  const current = resizing.value;
  if (!current || current.pointerId !== event.pointerId || current.length <= 0)
    return;
  event.preventDefault();
  // 极窄区域仍允许继续调整，同时避免一侧被压到完全不可见。
  const minimum = Math.max(0.1, Math.min(0.45, 80 / current.length));
  const point = current.axis === "horizontal" ? event.clientX : event.clientY;
  workspace.resizeSplit(
    current.id,
    Math.max(
      minimum,
      Math.min(1 - minimum, (point - current.start) / current.length),
    ),
  );
}
function stopResize() {
  const current = resizing.value;
  resizing.value = undefined;
  if (current?.element.hasPointerCapture(current.pointerId))
    current.element.releasePointerCapture(current.pointerId);
}
function resizeKey(event: KeyboardEvent, divider: SplitDivider) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    stopResize();
    return;
  }
  const horizontal = divider.axis === "horizontal";
  const forward = horizontal ? "ArrowRight" : "ArrowDown";
  const backward = horizontal ? "ArrowLeft" : "ArrowUp";
  if (![forward, backward, "Home", "End"].includes(event.key)) return;
  event.preventDefault();
  event.stopPropagation();
  workspace.resizeSplit(
    divider.id,
    event.key === "Home"
      ? 0.1
      : event.key === "End"
        ? 0.9
        : divider.ratio + (event.key === forward ? 0.05 : -0.05),
  );
}
watch(() => workspace.activeId, stopResize);
onMounted(() => window.addEventListener("blur", stopResize));
onBeforeUnmount(() => {
  stopResize();
  window.removeEventListener("blur", stopResize);
});
</script>
<template>
  <div
    ref="stage"
    :id="
      workspace.activeGroup
        ? 'workspace-panel-' + workspace.activeGroup.id
        : undefined
    "
    class="terminal-layout"
    role="tabpanel"
    :aria-labelledby="
      workspace.activeGroup
        ? 'workspace-tab-' + workspace.activeGroup.id
        : undefined
    "
    :class="{ 'is-resizing': !!resizing, 'is-dragging-tab': !!workspace.drag }"
  >
    <!-- 所有宿主始终在这个平面列表中；布局树只计算矩形，绝不递归挂载/移动 xterm。 -->
    <TerminalPane
      v-for="tab in workspace.tabs"
      :key="tab.terminalId"
      :tab="tab"
      :active="workspace.active?.terminalId === tab.terminalId"
      :broadcast="!!workspace.activeGroup?.broadcast"
      :visible="geometry.panes.has(tab.terminalId)"
      :split="
        !!workspace.activeGroup && workspace.activeGroup.root.kind === 'split'
      "
      :style="rectStyle(geometry.panes.get(tab.terminalId))"
      @focus="workspace.focusTerminal(tab.terminalId)"
      @close="emit('close', tab.terminalId)"
      @menu="emit('menu', $event, tab.terminalId)"
    />
    <div
      v-for="divider in geometry.dividers"
      :key="divider.id"
      class="terminal-divider"
      :class="[divider.axis, { resizing: resizing?.id === divider.id }]"
      :style="
        divider.axis === 'horizontal'
          ? {
              left: divider.position + '%',
              top: divider.bounds.top + '%',
              height: divider.bounds.height + '%',
            }
          : {
              top: divider.position + '%',
              left: divider.bounds.left + '%',
              width: divider.bounds.width + '%',
            }
      "
      role="separator"
      tabindex="0"
      :aria-label="
        divider.axis === 'horizontal' ? '调整左右终端宽度' : '调整上下终端高度'
      "
      :aria-orientation="
        divider.axis === 'horizontal' ? 'vertical' : 'horizontal'
      "
      :aria-valuenow="Math.round(divider.ratio * 100)"
      :aria-valuemin="10"
      :aria-valuemax="90"
      @pointerdown="startResize($event, divider)"
      @pointermove="moveResize"
      @pointerup="stopResize"
      @pointercancel="stopResize"
      @lostpointercapture="stopResize"
      @keydown="resizeKey($event, divider)"
    />
    <div
      v-if="preview"
      class="terminal-drop-preview"
      :style="rectStyle(preview)"
      aria-hidden="true"
    />
  </div>
</template>

<style scoped>
.terminal-layout {
  position: absolute;
  inset: 0;
  overflow: hidden;
}
.terminal-layout.is-dragging-tab {
  cursor: grabbing;
}
.terminal-layout.is-resizing :deep(.terminal-host) {
  pointer-events: none;
}
.terminal-divider {
  position: absolute;
  z-index: 5;
  touch-action: none;
}
.terminal-divider.horizontal {
  width: 6px;
  transform: translateX(-50%);
  cursor: col-resize;
}
.terminal-divider.vertical {
  height: 6px;
  transform: translateY(-50%);
  cursor: row-resize;
}
.terminal-divider:hover,
.terminal-divider:focus-visible,
.terminal-divider.resizing {
  background: var(--accent);
  outline: none;
}
.terminal-drop-preview {
  position: absolute;
  z-index: 10;
  border: 1px solid #18a058;
  background: #18a05840;
  pointer-events: none;
}
.terminal-layout.is-dragging-tab :deep(.terminal-pane-title) {
  cursor: grabbing;
}
</style>

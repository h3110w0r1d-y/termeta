<script setup lang="ts">
import { t } from "../../i18n";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { NButton, NIcon, useThemeVars } from "naive-ui";
import {
  IconChevronLeft,
  IconChevronRight,
  IconChevronUp,
  IconChevronDown,
} from "@tabler/icons-vue";
import TerminalPane from "../terminal/TerminalPane.vue";
import { layoutGeometry, type LayoutRect, type SplitDivider } from "./layout";
import { useWorkbenchStore } from "./store";
const emit = defineEmits<{
  close: [terminalId: string];
  menu: [event: MouseEvent | KeyboardEvent, terminalId: string];
}>();
const workbench = useWorkbenchStore();
const themeVars = useThemeVars();
const stage = ref<HTMLElement>();
const geometry = computed(() =>
  workbench.activeGroup
    ? layoutGeometry(workbench.activeGroup.root)
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
  const drop = workbench.dropTarget;
  if (
    !workbench.drag ||
    drop?.kind !== "pane" ||
    !workbench.canSplit(workbench.drag.source, drop.terminalId)
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
  if (event.button !== 0 || !event.isPrimary || workbench.drag || !stage.value)
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
  workbench.resizeSplit(
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
  workbench.resizeSplit(
    divider.id,
    event.key === "Home"
      ? 0.1
      : event.key === "End"
        ? 0.9
        : divider.ratio + (event.key === forward ? 0.05 : -0.05),
  );
}
watch(() => workbench.activeId, stopResize);
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
      workbench.activeGroup
        ? 'tab-panel-' + workbench.activeGroup.id
        : undefined
    "
    class="terminal-layout"
    :style="{
      '--divider-color': themeVars.borderColor,
      '--divider-text': themeVars.textColor2,
      '--divider-active': themeVars.primaryColor,
    }"
    role="tabpanel"
    :aria-labelledby="
      workbench.activeGroup ? 'app-tab-' + workbench.activeGroup.id : undefined
    "
    :class="{ 'is-resizing': !!resizing, 'is-dragging-tab': !!workbench.drag }"
  >
    <!-- 所有宿主始终在这个平面列表中；布局树只计算矩形，绝不递归挂载/移动 xterm。 -->
    <TerminalPane
      v-for="terminal in workbench.terminals"
      :key="terminal.terminalId"
      :terminal="terminal"
      :active="workbench.activeTerminal?.terminalId === terminal.terminalId"
      :broadcast="!!workbench.activeGroup?.broadcast"
      :visible="geometry.panes.has(terminal.terminalId)"
      :split="
        !!workbench.activeGroup && workbench.activeGroup.root.kind === 'split'
      "
      :style="rectStyle(geometry.panes.get(terminal.terminalId))"
      @focus="workbench.focusTerminal(terminal.terminalId)"
      @close="emit('close', terminal.terminalId)"
      @menu="emit('menu', $event, terminal.terminalId)"
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
        divider.axis === 'horizontal'
          ? t('workbench.layout.resizeHorizontal')
          : t('workbench.layout.resizeVertical')
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
    >
      <NButton
        class="terminal-divider-handle"
        :focusable="false"
        :tabindex="-1"
        size="tiny"
        :theme-overrides="{
          color: themeVars.bodyColor,
          colorHover: themeVars.bodyColor,
          colorPressed: themeVars.bodyColor,
          colorFocus: themeVars.bodyColor,
          textColor: themeVars.primaryColor,
          textColorHover: themeVars.primaryColor,
          textColorPressed: themeVars.primaryColor,
          textColorFocus: themeVars.primaryColor,
        }"
        :aria-label="
          divider.axis === 'horizontal'
            ? t('workbench.layout.swapHorizontal')
            : t('workbench.layout.swapVertical')
        "
        :title="
          divider.axis === 'horizontal'
            ? t('workbench.layout.swapHorizontal')
            : t('workbench.layout.swapVertical')
        "
        @pointerdown.stop.prevent
        @mousedown.stop.prevent
        @click.stop="workbench.swapSplitRegions(divider.id)"
        @keydown.stop
      >
        <span class="swap-triangles" aria-hidden="true">
          <NIcon :size="11">
            <IconChevronRight v-if="divider.axis === 'horizontal'" />
            <IconChevronDown v-else />
          </NIcon>
          <NIcon :size="11">
            <IconChevronLeft v-if="divider.axis === 'horizontal'" />
            <IconChevronUp v-else />
          </NIcon>
        </span>
      </NButton>
    </div>
    <div
      v-if="preview"
      class="terminal-drop-preview"
      :style="{
        ...rectStyle(preview),
        '--drop-primary': themeVars.primaryColor,
      }"
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
/* 按钮各状态通过库主题覆盖保持不透明，避免悬停时透出分隔线。 */
/* 命中区域保留六像素，三像素主体与中点三角手柄均仅在悬停时显示。 */
.terminal-divider::before {
  content: "";
  position: absolute;
  z-index: 0;
  background: var(--divider-active);
  visibility: hidden;
}
.terminal-divider.horizontal::before {
  inset: 0 auto 0 50%;
  width: 3px;
  transform: translateX(-50%);
}
.terminal-divider.vertical::before {
  inset: 50% 0 auto 0;
  height: 3px;
  transform: translateY(-50%);
}
.terminal-divider-handle {
  position: absolute;
  z-index: 1;
  left: 50%;
  top: 50%;
  display: flex;
  visibility: hidden;
  align-items: center;
  justify-content: center;
  transform: translate(-50%, -50%);
  border: 1px solid var(--divider-color);
  border-radius: 5px;
  color: var(--divider-text);
}
.terminal-divider.horizontal .terminal-divider-handle {
  width: 20px;
  height: 24px;
  padding: 0;
  cursor: pointer;
}
.terminal-divider.vertical .terminal-divider-handle {
  width: 24px;
  height: 20px;
  padding: 0;
  cursor: pointer;
}
.swap-triangles {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0;
  line-height: 1;
}
/* 上下交换将排列方向一起转为横向，不能只替换三角形朝向。 */
.terminal-divider.vertical .swap-triangles {
  flex-direction: row;
}
.terminal-divider.vertical .swap-triangles > :first-child {
  transform: translate(1px, -2px);
}
.terminal-divider.vertical .swap-triangles > :last-child {
  margin-top: 0;
  margin-left: -4px;
  transform: translate(-1px, 2px);
}
.swap-triangles > :first-child {
  transform: translateX(-0.5px);
}
.swap-triangles > :last-child {
  margin-top: -3px;
  transform: translateX(0.5px);
}
.terminal-divider:focus-visible {
  outline: none;
}
.terminal-divider:hover::before {
  visibility: visible;
}
.terminal-divider:hover .terminal-divider-handle {
  visibility: visible;
  border-color: var(--divider-active);
  color: var(--divider-active);
}
.terminal-drop-preview {
  position: absolute;
  z-index: 10;
  border: 1px solid var(--drop-primary);
  background: color-mix(in srgb, var(--drop-primary) 25%, transparent);
  pointer-events: none;
}
.terminal-layout.is-dragging-tab :deep(.terminal-pane-title) {
  cursor: grabbing;
}
</style>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { NIcon, useThemeVars } from "naive-ui";
import AppButton from "../../components/ui/AppButton.vue";
import type { TerminalTab } from "../workspace/store";
import { stateLabels } from "../workspace/store";
import { schemeForTab, terminalRegistry } from "./registry";
import { terminalDisplayTitle } from "./title";
import SftpPanel from "../files/SftpPanel.vue";
const props = defineProps<{
  tab: TerminalTab;
  active: boolean;
  visible: boolean;
  split: boolean;
  broadcast: boolean;
}>();
const emit = defineEmits<{
  focus: [];
  close: [];
  menu: [event: MouseEvent | KeyboardEvent];
}>();
const themeVars = useThemeVars();
const background = computed(() => schemeForTab(props.tab).colors.background);
const displayTitle = computed(() =>
  terminalDisplayTitle(props.tab.title, props.tab.remoteTitle),
);
const host = ref<HTMLElement>();
const sftpMounted = ref(false);
watch(
  () => props.tab.sftpOpen,
  (open) => {
    if (open) sftpMounted.value = true;
  },
  { immediate: true },
);
onMounted(() => {
  if (host.value) terminalRegistry.mount(props.tab, host.value);
  if (props.active) terminalRegistry.focus(props.tab.terminalId);
});
watch(
  () => props.active,
  (active) => {
    // 键盘标签导航保留导航焦点；分屏焦点只路由到选中的子终端。
    if (active && !document.activeElement?.closest('[role="tablist"]'))
      terminalRegistry.focus(props.tab.terminalId);
  },
);
function menuKey(event: KeyboardEvent) {
  if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10"))
    emit("menu", event);
}
</script>
<template>
  <div
    :id="'terminal-panel-' + tab.terminalId"
    role="region"
    v-show="visible"
    class="terminal-pane"
    :class="{
      'split-pane': split,
      'focused-pane': split && active,
      'broadcast-pane': split && broadcast,
      'broadcast-peer': split && broadcast && !active,
    }"
    :data-terminal-pane="tab.terminalId"
    :aria-label="displayTitle"
    :style="{
      '--terminal-background': background,
      '--pane-focus-color': themeVars.primaryColor,
    }"
    @pointerdown.capture="emit('focus')"
    @focusin="emit('focus')"
  >
    <header v-if="split" class="terminal-pane-heading">
      <button
        type="button"
        class="terminal-pane-title"
        :data-drag-terminal="tab.terminalId"
        :title="displayTitle + ' · ' + stateLabels[tab.state]"
        @click="terminalRegistry.focus(tab.terminalId)"
        @contextmenu="emit('menu', $event)"
        @keydown="menuKey"
      >
        <i class="status-dot" :class="tab.state" aria-hidden="true" />
        <span class="terminal-pane-title-text">{{ displayTitle }}</span>
      </button>
      <AppButton
        class="icon-button pane-broadcast"
        :variant="broadcast ? 'primary' : 'default'"
        :aria-pressed="broadcast"
        :aria-label="broadcast ? '关闭工作区广播' : '开启工作区广播'"
        :title="broadcast ? '关闭广播' : '广播输入到工作区全部已连接终端'"
        @click="terminalRegistry.toggleBroadcast()"
      >
        <NIcon :size="16" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none" focusable="false">
            <path
              d="M8 12h5m0-6v12m0-12h4m-4 6h4m-4 6h4"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
            <rect
              x="3"
              y="9.5"
              width="5"
              height="5"
              rx="1"
              stroke="currentColor"
              stroke-width="1.6"
            />
            <path
              d="M17 4h4v4h-4zM17 10h4v4h-4zM17 16h4v4h-4z"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linejoin="round"
            />
          </svg>
        </NIcon>
      </AppButton>
      <AppButton
        class="icon-button pane-close"
        :aria-label="'关闭终端 ' + tab.title"
        :title="'关闭终端 ' + tab.title"
        @click="emit('close')"
      >
        <NIcon :size="14" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none" focusable="false">
            <path
              d="m6 6 12 12M18 6 6 18"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
            />
          </svg>
        </NIcon>
      </AppButton>
    </header>
    <div class="terminal-pane-body">
      <div ref="host" class="terminal-host" />
      <SftpPanel
        v-if="sftpMounted"
        v-show="!!tab.sftpOpen && !split"
        :tab="tab"
      />
    </div>
  </div>
</template>

<style scoped>
.terminal-pane {
  position: absolute;
  inset: 0;
}
.terminal-host {
  height: auto;
  margin: 14px 8px 10px 14px;
  overflow: hidden;
  background: var(--terminal-background);
}
/* 只隔离滚动链，滚轮与全屏程序鼠标事件仍交给 xterm 处理。 */
.terminal-host :deep(.xterm-viewport),
.terminal-host :deep(.xterm-scrollable-element) {
  overscroll-behavior: none;
}
.terminal-pane {
  inset: auto;
  display: flex;
  flex-direction: column;
  background: var(--terminal-background);
}
.terminal-pane.split-pane {
  /* 外框只占位，状态线内缩到边框内侧，避开视口裁剪边界的小数像素。 */
  border: 1px solid transparent;
  isolation: isolate;
}
.terminal-pane.split-pane::after {
  content: "";
  position: absolute;
  inset: 0;
  z-index: 1;
  border: 1px var(--pane-frame-style, solid)
    var(--pane-frame-color, var(--border));
  pointer-events: none;
}
.terminal-pane.focused-pane {
  --pane-frame-color: var(--pane-focus-color);
}
.terminal-pane .terminal-host {
  flex: 1;
  min-height: 0;
}
.terminal-pane.split-pane .terminal-host {
  margin: 0 6px 6px;
}
.terminal-pane-heading {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  gap: 4px;
  height: 26px;
  margin: 4px 6px;
}
.terminal-pane-title {
  /* 状态点与文字按容器居中，避免行内基线使圆点偏下。 */
  display: flex;
  align-items: center;
  flex: 1;
  min-width: 0;
  padding: 0 4px;
  overflow: hidden;
  white-space: nowrap;
  text-align: left;
  font: inherit;
  font-size: 12px;
  color: var(--text);
  background: transparent;
  border: 0;
  cursor: pointer;
}
.terminal-pane-title-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.terminal-pane-title:focus-visible {
  outline: 1px solid var(--pane-focus-color);
}
.terminal-pane-heading .pane-close,
.terminal-pane-heading .pane-broadcast {
  width: 24px;
  height: 24px;
  padding: 0;
}

/* 窗格仅标题可拖动，终端正文仍保留原生xterm选区与鼠标协议。 */
.terminal-pane-title[data-drag-terminal] {
  touch-action: none;
  cursor: grab;
}

/* 广播与焦点共用内部状态线，不改变终端尺寸，也不拦截选区或拖动。 */
.terminal-pane.broadcast-pane {
  --pane-frame-color: var(--pane-focus-color);
}
.terminal-pane.broadcast-peer {
  --pane-frame-style: dashed;
}

.terminal-pane-body {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.terminal-pane-body > .terminal-host {
  width: 0;
  min-width: 0;
}
</style>

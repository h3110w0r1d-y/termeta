<script setup lang="ts">
import { computed, ref } from "vue";
import { NIcon } from "naive-ui";
import { IconFileText, IconX } from "@tabler/icons-vue";
import { t } from "@/i18n";
import { stateLabels } from "@/modules/sessions/presentation";
import { isMac } from "@/services/commands/registry";
import type { TabBarItem } from "../types";

const props = defineProps<{
  tab: TabBarItem;
  selected: boolean;
  dragging: boolean;
}>();
const emit = defineEmits<{
  activate: [];
  close: [];
  doubleClick: [];
  menu: [event: MouseEvent];
  dragStart: [event: PointerEvent];
}>();
const title = computed(() =>
  props.tab.kind === "log"
    ? props.tab.title
    : props.tab.title + " · " + t(stateLabels[props.tab.state]),
);
const label = computed(() =>
  props.tab.kind === "log"
    ? props.tab.title
    : props.tab.title +
      (props.tab.split
        ? " · " +
          t("workbench.terminalCount", { count: props.tab.terminalIds.length })
        : "") +
      " · " +
      t(stateLabels[props.tab.state]),
);
const closeLabel = computed(() =>
  t("workbench.actions.closeNamed", { name: props.tab.title }),
);
const titleButton = ref<HTMLButtonElement>();
function activate() {
  // 状态标记和标签留白共用激活入口，日志标签也保留原生按钮的焦点行为。
  titleButton.value?.focus({ preventScroll: true });
  emit("activate");
}
</script>
<template>
  <div
    class="app-tab-item"
    :class="{ selected, dragging, 'platform-mac': isMac }"
    :data-sort-tab="tab.kind === 'terminal' ? tab.id : undefined"
    role="presentation"
    :title="title"
    @click="activate"
    @dblclick="tab.kind === 'terminal' && emit('doubleClick')"
    @mousedown.middle.prevent
    @auxclick.middle.prevent.stop="emit('close')"
    @pointerdown="tab.kind === 'terminal' && emit('dragStart', $event)"
    @contextmenu.prevent="tab.kind === 'terminal' && emit('menu', $event)"
  >
    <NIcon
      v-if="tab.kind === 'log'"
      class="app-tab-indicator"
      :size="16"
      aria-hidden="true"
    >
      <IconFileText />
    </NIcon>
    <i
      v-else
      class="app-tab-indicator status-dot"
      :class="tab.state"
      aria-hidden="true"
    />
    <button
      ref="titleButton"
      :id="'app-tab-' + tab.id"
      type="button"
      role="tab"
      class="app-tab"
      :data-app-tab="tab.id"
      :aria-selected="selected"
      :tabindex="selected ? 0 : -1"
      :aria-controls="'tab-panel-' + tab.id"
      :aria-label="label"
    >
      <span class="app-tab-title">{{ tab.title }}</span>
    </button>
    <button
      type="button"
      class="app-tab-close"
      tabindex="-1"
      :aria-label="closeLabel"
      :title="closeLabel"
      @click.stop="emit('close')"
      @dblclick.stop
    >
      <NIcon :size="16" aria-hidden="true"><IconX /></NIcon>
    </button>
  </div>
</template>
<style scoped>
.app-tab-item {
  display: grid;
  grid-template-areas: "indicator title close";
  grid-template-columns: 16px minmax(0, 1fr) var(--tab-close-size);
  align-items: center;
  column-gap: 6px;
  flex: 0 0 auto;
  min-width: 0;
  max-width: var(--app-tab-max-width, 260px);
  height: var(--tab-height);
  touch-action: none;
  cursor: pointer;
  padding-inline: 6px;
  color: var(--muted);
  border-radius: var(--app-tab-radius);
}
.app-tab-item.platform-mac {
  grid-template-areas: "close title indicator";
  grid-template-columns: var(--tab-close-size) minmax(0, 1fr) 16px;
}
.app-tab,
.app-tab-close {
  appearance: none;
  border: 0;
  border-radius: var(--app-tab-radius);
  background: transparent;
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.app-tab {
  grid-area: title;
  display: flex;
  align-items: center;
  min-width: 0;
  height: var(--tab-height);
  padding: 0;
  line-height: 24px;
  white-space: nowrap;
  text-align: left;
}
.app-tab-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.app-tab-indicator {
  grid-area: indicator;
  justify-self: center;
  margin: 0;
}
.app-tab-item:not(.selected):not(:hover) {
  background: var(--app-tab-background);
}
.app-tab-item:hover {
  background: var(--app-tab-hover);
}
.app-tab-item.selected {
  background: var(--app-tab-active);
  color: var(--app-tab-text);
}
.app-tab-item:has(.app-tab:focus-visible),
.app-tab-close:focus-visible {
  outline: 2px solid var(--app-tab-text);
  outline-offset: -2px;
}
.app-tab:focus-visible {
  outline: none;
}
.app-tab-close {
  grid-area: close;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  line-height: 0;
  width: var(--tab-close-size);
  height: var(--tab-close-size);
  margin: 0;
}
.app-tab-close:hover {
  background: var(--app-tab-close-hover);
  color: var(--app-tab-text);
}
.selected .app-tab-close:hover {
  background: var(--app-tab-close-selected-hover);
}
.app-tab-close .n-icon,
.app-tab-close svg {
  display: block;
}
.app-tab-item.dragging {
  opacity: 0.45;
}
</style>

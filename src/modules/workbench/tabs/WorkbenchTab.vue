<script setup lang="ts">
import { computed } from "vue";
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
</script>
<template>
  <div
    class="app-tab-item"
    :class="{ selected, dragging, 'platform-mac': isMac }"
    :data-sort-tab="tab.kind === 'terminal' ? tab.id : undefined"
    role="presentation"
    @mousedown.middle.prevent
    @auxclick.middle.prevent.stop="emit('close')"
    @pointerdown="tab.kind === 'terminal' && emit('dragStart', $event)"
    @contextmenu.prevent="tab.kind === 'terminal' && emit('menu', $event)"
  >
    <button
      :id="'app-tab-' + tab.id"
      type="button"
      role="tab"
      class="app-tab"
      :data-app-tab="tab.id"
      :aria-selected="selected"
      :tabindex="selected ? 0 : -1"
      :aria-controls="'tab-panel-' + tab.id"
      :aria-label="label"
      :title="title"
      @click="emit('activate')"
      @dblclick="tab.kind === 'terminal' && emit('doubleClick')"
    >
      <NIcon v-if="tab.kind === 'log'" :size="16" aria-hidden="true">
        <IconFileText />
      </NIcon>
      <i v-else class="status-dot" :class="tab.state" aria-hidden="true" />
      <span class="app-tab-title">{{ tab.title }}</span>
    </button>
    <button
      type="button"
      class="app-tab-close"
      tabindex="-1"
      :aria-label="closeLabel"
      :title="closeLabel"
      @click="emit('close')"
    >
      <NIcon :size="16" aria-hidden="true"><IconX /></NIcon>
    </button>
  </div>
</template>
<style scoped>
.app-tab-item {
  display: flex;
  flex: 0 0 auto;
  max-width: var(--app-tab-max-width, 260px);
  height: var(--tab-height);
  touch-action: none;
  padding-right: 6px;
  color: var(--muted);
  border-radius: var(--app-tab-radius);
}
.app-tab-item.platform-mac {
  padding-right: 0;
  padding-left: 6px;
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
  display: flex;
  align-items: center;
  flex: 1;
  gap: 4px;
  min-width: 0;
  height: var(--tab-height);
  padding: var(--tab-padding-block) 12px;
  line-height: 24px;
  white-space: nowrap;
  text-align: left;
}
.app-tab-title {
  overflow: hidden;
  text-overflow: ellipsis;
}
.platform-mac .status-dot {
  order: 1;
  margin: 0 0 0 auto;
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
.app-tab:focus-visible,
.app-tab-close:focus-visible {
  outline: 2px solid var(--app-tab-text);
  outline-offset: -2px;
}
.app-tab-close {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  line-height: 0;
  flex: 0 0 auto;
  align-self: center;
  width: var(--tab-close-size);
  height: var(--tab-close-size);
  margin: 0;
}
.platform-mac .app-tab-close {
  order: -1;
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

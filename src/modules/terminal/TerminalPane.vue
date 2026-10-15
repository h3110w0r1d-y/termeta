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

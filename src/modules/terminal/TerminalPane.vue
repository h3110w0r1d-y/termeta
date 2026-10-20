<script setup lang="ts">
import { t } from "@/i18n";
import { IconGitBranch, IconX } from "@tabler/icons-vue";
import { computed, onMounted, ref, watch } from "vue";
import {
  NConfigProvider,
  NIcon,
  darkTheme,
  lightTheme,
  type GlobalThemeOverrides,
} from "naive-ui";
import AppButton from "@/components/ui/AppButton.vue";
import { useWorkbenchStore, type TerminalItem } from "../workbench/store";
import { stateLabels } from "../workbench/store";
import { schemeForTerminal, terminalRegistry } from "./registry";
import { terminalBaseTitle, terminalDisplayTitle } from "./title";
import SftpPanel from "../files/SftpPanel.vue";
import { colorOverlay, isLightBackground } from "./appearance";
const props = defineProps<{
  terminal: TerminalItem;
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
const workbench = useWorkbenchStore();
const broadcastCount = computed(() =>
  workbench.broadcastTargetCount(props.terminal.terminalId),
);
const participatesInBroadcast = computed(
  () => props.broadcast && !props.terminal.sshCommand,
);
const broadcastTitle = computed(() =>
  props.terminal.sshCommand
    ? t("workbench.broadcast.commandExcluded")
    : props.broadcast
      ? t("workbench.broadcast.disable", {
          broadcastCount: broadcastCount.value,
        })
      : broadcastCount.value < 2
        ? t("workbench.broadcast.minimumTerminals")
        : t("workbench.broadcast.enable", {
            broadcastCount: broadcastCount.value,
          }),
);
const colors = computed(() => schemeForTerminal(props.terminal).colors);
const uiColor = computed(() => colors.value.ui || colors.value.foreground);
const headingTheme = computed(() =>
  isLightBackground(colors.value.background) ? lightTheme : darkTheme,
);
const headingThemeOverrides = computed<GlobalThemeOverrides>(() => ({
  // 只作用于标题栏，正文与 SFTP 不继承局部按钮主题。
  Button: {
    textColor: "var(--pane-button-color)",
    textColorHover: "var(--pane-button-color)",
    textColorPressed: "var(--pane-button-color)",
    textColorFocus: "var(--pane-button-color)",
    colorPrimary: "var(--pane-button-color)",
    colorQuaternaryHover: colorOverlay(uiColor.value, 20),
    colorQuaternaryPressed: colorOverlay(uiColor.value, 20),
  },
}));
const displayTitle = computed(() =>
  terminalDisplayTitle(
    terminalBaseTitle(props.terminal),
    props.terminal.remoteTitle,
  ),
);
const host = ref<HTMLElement>();
const sftpMounted = ref(false);
watch(
  () => props.terminal.sftpOpen,
  (open) => {
    if (open) sftpMounted.value = true;
  },
  { immediate: true },
);
onMounted(() => {
  if (host.value) terminalRegistry.mount(props.terminal, host.value);
  if (props.active) terminalRegistry.focus(props.terminal.terminalId);
});
watch(
  () => props.active,
  (active) => {
    // 键盘标签导航保留导航焦点；分屏焦点只路由到选中的子终端。
    if (active && !document.activeElement?.closest('[role="tablist"]'))
      terminalRegistry.focus(props.terminal.terminalId);
  },
);
function menuKey(event: KeyboardEvent) {
  if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10"))
    emit("menu", event);
}
</script>
<template>
  <div
    :id="'terminal-panel-' + terminal.terminalId"
    role="region"
    v-show="visible"
    class="terminal-pane"
    :class="{
      'split-pane': split,
      'focused-pane': split && active,
      'broadcast-pane': split && participatesInBroadcast,
      'broadcast-peer': split && participatesInBroadcast && !active,
    }"
    :data-terminal-pane="terminal.terminalId"
    :aria-label="displayTitle"
    :style="{
      '--terminal-background': colors.background,
      '--pane-focus-color': colors.foreground,
      '--pane-frame-default-color': colorOverlay(uiColor, 20),
      '--pane-foreground': colors.foreground,
      '--pane-muted-color': uiColor,
      '--pane-heading-color': active ? colors.foreground : uiColor,
      '--pane-button-color': uiColor,
      '--pane-button-selected-background': colorOverlay(colors.foreground, 10),
    }"
    @pointerdown.capture="emit('focus')"
    @focusin="emit('focus')"
  >
    <NConfigProvider
      v-if="split"
      abstract
      :theme="headingTheme"
      :theme-overrides="headingThemeOverrides"
    >
      <header class="terminal-pane-heading">
        <button
          type="button"
          class="terminal-pane-title"
          :data-drag-terminal="terminal.terminalId"
          :title="displayTitle + ' · ' + t(stateLabels[terminal.state])"
          @click="terminalRegistry.focus(terminal.terminalId)"
          @contextmenu="emit('menu', $event)"
          @keydown="menuKey"
        >
          <i class="status-dot" :class="terminal.state" aria-hidden="true" />
          <span class="terminal-pane-title-text">{{ displayTitle }}</span>
          <span v-if="terminal.sshCommand" class="pane-command-label">
            {{ t("terminal.titles.command") }}
          </span>
        </button>
        <AppButton
          class="icon-button pane-broadcast"
          :variant="participatesInBroadcast ? 'primary' : 'default'"
          :aria-pressed="participatesInBroadcast"
          :aria-label="broadcastTitle"
          :title="broadcastTitle"
          :disabled="
            !!terminal.sshCommand || (!broadcast && broadcastCount < 2)
          "
          @click="terminalRegistry.toggleBroadcast()"
        >
          <NIcon :size="16" aria-hidden="true">
            <IconGitBranch />
          </NIcon>
        </AppButton>
        <AppButton
          class="icon-button pane-close"
          :aria-label="
            t('terminal.actions.closeNamed', {
              name: terminalBaseTitle(terminal),
            })
          "
          :title="
            t('terminal.actions.closeNamed', {
              name: terminalBaseTitle(terminal),
            })
          "
          @click="emit('close')"
        >
          <NIcon :size="14" aria-hidden="true">
            <IconX />
          </NIcon>
        </AppButton>
      </header>
    </NConfigProvider>
    <div class="terminal-pane-body">
      <div ref="host" class="terminal-host" />
      <SftpPanel
        v-if="sftpMounted"
        v-show="!!terminal.sftpOpen && !split"
        :terminal="terminal"
      />
    </div>
  </div>
</template>

<style scoped>
.pane-command-label {
  color: var(--pane-muted-color);
  font-size: 11px;
  flex-shrink: 0;
  margin-left: 6px;
}
.terminal-pane {
  position: absolute;
  inset: 0;
}
.terminal-host {
  height: auto;
  margin: 0 8px 10px 10px;
  margin-top: var(--terminal-host-top-margin, 0);
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
    var(--pane-frame-color, var(--pane-frame-default-color));
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
  margin-top: var(--terminal-host-top-margin, 0);
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
  color: var(--pane-heading-color);
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

/* 与 Header 共用三色层级，按钮悬停和广播启用态使用当前终端前景色。 */
.terminal-pane-title:hover {
  color: var(--pane-foreground);
}
.terminal-pane-heading :deep(.n-button:hover),
.terminal-pane-heading :deep(.n-button:focus-visible),
.terminal-pane-heading :deep(.n-button:active),
.terminal-pane-heading .pane-broadcast[aria-pressed="true"] {
  --pane-button-color: var(--pane-foreground);
}
.terminal-pane-heading .pane-broadcast[aria-pressed="true"] {
  background: var(--pane-button-selected-background);
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

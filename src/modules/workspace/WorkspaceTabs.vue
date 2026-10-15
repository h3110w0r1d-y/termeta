<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { NIcon, useThemeVars } from "naive-ui";
import AppButton from "../../components/ui/AppButton.vue";
import { stateLabels, type TerminalTab } from "./store";
const props = defineProps<{
  tabs: TerminalTab[];
  activeId: string;
  position: "top" | "left";
  connected: number;
}>();
const emit = defineEmits<{
  activate: [id: string];
  focusTerminal: [id: string];
  close: [id: string];
  menu: [event: MouseEvent | KeyboardEvent, id: string];
  create: [];
}>();
const themeVars = useThemeVars();
const root = ref<HTMLElement>();
const fixedTabs = [
  { id: "home", title: "首页", panel: "page-home" },
  { id: "sftp", title: "SFTP", panel: "page-sftp" },
];
function selectedElement() {
  return root.value?.querySelector<HTMLElement>(
    '[role="tab"][aria-selected="true"]',
  );
}
function activate(id: string) {
  emit("activate", id);
  if (props.tabs.some((tab) => tab.terminalId === id))
    emit("focusTerminal", id);
}
async function onKey(event: KeyboardEvent) {
  const target = event.target as HTMLElement;
  if (target.getAttribute("role") !== "tab" || event.isComposing) return;
  const id = target.dataset.workspaceTab!;
  const session = props.tabs.some((tab) => tab.terminalId === id);
  if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) {
    if (session) emit("menu", event, id);
    return;
  }
  if (event.altKey || event.ctrlKey || event.metaKey) return;
  if (event.key === "Delete" && session) {
    event.preventDefault();
    emit("close", id);
    return;
  }
  const ids = [
    ...fixedTabs.map((tab) => tab.id),
    ...props.tabs.map((tab) => tab.terminalId),
  ];
  const index = ids.indexOf(id);
  const vertical = props.position === "left";
  let next: number;
  if (event.key === (vertical ? "ArrowDown" : "ArrowRight"))
    next = (index + 1) % ids.length;
  else if (event.key === (vertical ? "ArrowUp" : "ArrowLeft"))
    next = (index - 1 + ids.length) % ids.length;
  else if (event.key === "Home") next = 0;
  else if (event.key === "End") next = ids.length - 1;
  else if (event.key === "Enter" || event.key === " ") next = index;
  else return;
  event.preventDefault();
  // 键盘导航保留标签焦点，点击终端标签才将焦点交给终端。
  emit("activate", ids[next]!);
  await nextTick();
  selectedElement()?.focus({ preventScroll: true });
}
watch(
  () => [props.activeId, props.position, props.tabs.length],
  () =>
    selectedElement()?.scrollIntoView({ block: "nearest", inline: "nearest" }),
  { flush: "post" },
);
</script>
<template>
  <nav
    ref="root"
    class="app-tabs"
    aria-label="工作区标签"
    :style="{
      '--workspace-tab-hover': themeVars.buttonColor2,
      '--workspace-tab-active': themeVars.buttonColor2Hover,
      '--workspace-tab-text': themeVars.textColor1,
      '--workspace-tab-close-hover': themeVars.closeColorHover,
      '--workspace-tab-radius': themeVars.borderRadius,
    }"
  >
    <div class="tab-brand" title="Termvexa">›_</div>
    <!-- 单一标签列表维持固定页与会话的统一导航，固定区不参与会话滚动。 -->
    <div
      class="workspace-tablist"
      role="tablist"
      aria-label="工作区"
      :aria-orientation="position === 'left' ? 'vertical' : 'horizontal'"
      @keydown="onKey"
    >
      <div class="fixed-tabs" role="presentation">
        <button
          v-for="tab in fixedTabs"
          :key="tab.id"
          type="button"
          role="tab"
          class="workspace-tab"
          :id="'workspace-tab-' + tab.id"
          :data-workspace-tab="tab.id"
          :aria-selected="activeId === tab.id"
          :tabindex="activeId === tab.id ? 0 : -1"
          :aria-controls="tab.panel"
          @click="activate(tab.id)"
        >
          {{ tab.title }}
        </button>
      </div>
      <div class="session-tabs" role="presentation">
        <div
          v-for="tab in tabs"
          :key="tab.terminalId"
          class="workspace-tab-item"
          :class="{ selected: activeId === tab.terminalId }"
          role="presentation"
          @contextmenu="emit('menu', $event, tab.terminalId)"
        >
          <button
            type="button"
            role="tab"
            class="workspace-tab"
            :id="'workspace-tab-' + tab.terminalId"
            :data-workspace-tab="tab.terminalId"
            :aria-selected="activeId === tab.terminalId"
            :tabindex="activeId === tab.terminalId ? 0 : -1"
            :aria-controls="'terminal-panel-' + tab.terminalId"
            :aria-label="tab.title + ' · ' + stateLabels[tab.state]"
            :title="tab.title + ' · ' + stateLabels[tab.state]"
            @click="activate(tab.terminalId)"
          >
            <i class="status-dot" :class="tab.state" aria-hidden="true" /><span
              class="workspace-tab-title"
              >{{ tab.title }}</span
            >
          </button>
          <button
            type="button"
            class="workspace-tab-close"
            tabindex="-1"
            :aria-label="'关闭 ' + tab.title"
            :title="'关闭 ' + tab.title"
            @click="emit('close', tab.terminalId)"
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
          </button>
        </div>
      </div>
    </div>
    <AppButton
      class="icon-button new-tab"
      aria-label="新建 SSH 连接"
      title="新建 SSH 连接"
      @click="emit('create')"
      >＋</AppButton
    >
    <div class="workspace-tabs-spacer" aria-hidden="true" />
    <span class="connection-count"
      ><i class="status-dot" :class="{ connected: connected > 0 }" />{{
        connected
      }}
      个连接</span
    >
  </nav>
</template>

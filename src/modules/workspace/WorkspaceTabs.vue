<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { NIcon, useThemeVars } from "naive-ui";
import AppButton from "../../components/ui/AppButton.vue";
import { stateLabels, useWorkspace, type WorkspaceTabItem } from "./store";
import { useTabDrag } from "./useTabDrag";
const props = defineProps<{
  tabs: WorkspaceTabItem[];
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
const workspace = useWorkspace();
const tabDrag = useTabDrag((id) => emit("focusTerminal", id));
const root = ref<HTMLElement>();
// 最后一个标签后方与空白区是同一插入位置，只使用新建按钮前的提示线。
// 保留原命中类型，避免影响悬停标签切页和实际插入逻辑。
const dropAtEnd = computed(() => {
  const target = workspace.dropTarget;
  return (
    target?.kind === "tab-end" ||
    (target?.kind === "tab" &&
      !target.before &&
      target.tabId === props.tabs[props.tabs.length - 1]?.id)
  );
});
const dragTitle = computed(() => {
  const source = workspace.drag?.source;
  if (!source) return "";
  return source.kind === "pane"
    ? workspace.tabs.find((tab) => tab.terminalId === source.terminalId)?.title
    : props.tabs.find((tab) => tab.id === source.tabId)?.title;
});
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
  if (props.tabs.some((tab) => tab.id === id)) emit("focusTerminal", id);
}
async function onKey(event: KeyboardEvent) {
  const target = event.target as HTMLElement;
  if (target.getAttribute("role") !== "tab" || event.isComposing) return;
  const id = target.dataset.workspaceTab!;
  const session = props.tabs.some((tab) => tab.id === id);
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
    ...props.tabs.map((tab) => tab.id),
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
    :class="{ 'is-dragging': !!workspace.drag }"
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
          :key="tab.id"
          class="workspace-tab-item"
          :class="{
            selected: activeId === tab.id,
            dragging:
              workspace.drag?.source.kind === 'tab' &&
              workspace.drag.source.tabId === tab.id,
            'drop-before':
              workspace.dropTarget?.kind === 'tab' &&
              workspace.dropTarget.tabId === tab.id &&
              workspace.dropTarget.before,
            'drop-after':
              workspace.dropTarget?.kind === 'tab' &&
              workspace.dropTarget.tabId === tab.id &&
              !workspace.dropTarget.before &&
              !dropAtEnd,
          }"
          :data-sort-tab="tab.id"
          @pointerdown="tabDrag.start($event, tab.id)"
          role="presentation"
          @contextmenu="emit('menu', $event, tab.id)"
        >
          <button
            type="button"
            role="tab"
            class="workspace-tab"
            :id="'workspace-tab-' + tab.id"
            :data-workspace-tab="tab.id"
            :aria-selected="activeId === tab.id"
            :tabindex="activeId === tab.id ? 0 : -1"
            :aria-controls="'workspace-panel-' + tab.id"
            :aria-label="
              tab.title +
              (tab.split ? ' · ' + tab.terminalIds.length + ' 个终端' : '') +
              ' · ' +
              stateLabels[tab.state]
            "
            :title="tab.title + ' · ' + stateLabels[tab.state]"
            @click="activate(tab.id)"
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
            @click="emit('close', tab.id)"
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
    <!-- 末尾标记独立于按钮，整片后方命中区只强调同一个插入位置。 -->
    <div class="workspace-tab-end" :class="{ 'drop-at-end': dropAtEnd }">
      <AppButton
        class="icon-button new-tab"
        aria-label="新建 SSH 连接"
        title="新建 SSH 连接"
        @click="emit('create')"
        >＋</AppButton
      >
    </div>
    <div
      v-if="workspace.drag"
      class="workspace-drag-label"
      :style="{
        left: workspace.drag.x + 12 + 'px',
        top: workspace.drag.y + 12 + 'px',
      }"
      aria-hidden="true"
    >
      {{ dragTitle }}
    </div>
    <div class="workspace-tabs-spacer" aria-hidden="true" />
    <span class="connection-count"
      ><i class="status-dot" :class="{ connected: connected > 0 }" />{{
        connected
      }}
      个连接</span
    >
  </nav>
</template>

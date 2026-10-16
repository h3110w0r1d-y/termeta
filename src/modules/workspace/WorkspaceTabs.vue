<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { NIcon, useThemeVars } from "naive-ui";
import { isMac } from "../../services/commands/registry";
import WindowControls from "../../components/WindowControls.vue";
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
  doubleClick: [id: string];
  menu: [event: MouseEvent | KeyboardEvent, id: string];
  create: [];
  windowError: [message: string];
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
    event.preventDefault();
    if (props.tabs.some((tab) => tab.id === id && !tab.split))
      emit("menu", event, id);
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
    :data-tauri-drag-region="position === 'top' ? '' : undefined"
    :class="{
      'is-dragging': !!workspace.drag,
      'tabs-vertical': position === 'left',
      'titlebar-mac': isMac && position === 'top',
    }"
    aria-label="工作区标签"
    :style="{
      '--workspace-tab-hover': themeVars.buttonColor2,
      '--workspace-tab-active': themeVars.buttonColor2Hover,
      '--workspace-tab-text': themeVars.textColor1,
      '--workspace-tab-close-hover': themeVars.closeColorHover,
      '--workspace-tab-radius': themeVars.borderRadius,
    }"
  >
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
          @contextmenu.prevent="!tab.split && emit('menu', $event, tab.id)"
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
            @dblclick="emit('doubleClick', tab.id)"
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
            <NIcon :size="position === 'top' ? 16 : 14" aria-hidden="true">
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
        aria-label="新建标签页"
        title="新建标签页 (Cmd/Ctrl+T)"
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
    <div
      class="workspace-tabs-spacer"
      :data-tauri-drag-region="position === 'top' ? '' : undefined"
      aria-hidden="true"
    />
    <span
      :data-tauri-drag-region="position === 'top' ? '' : undefined"
      class="connection-count"
      ><i class="status-dot" :class="{ connected: connected > 0 }" />{{
        connected
      }}
      个连接</span
    >
    <WindowControls
      v-if="position === 'top'"
      @error="emit('windowError', $event)"
    />
  </nav>
</template>

<style scoped>
.app-tabs {
  --tab-height: 32px;
  --tab-padding-block: 4px;
  --tab-close-size: 22px;
  display: flex;
  align-items: center;
  flex-shrink: 0;
  height: 54px;
  gap: 6px;
  padding: 0 12px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}
/* 工作区标签使用独立 DOM，固定页与会话共享导航但分开滚动。 */
.workspace-tablist {
  display: flex;
  align-items: center;
  flex: 0 1 auto;
  min-width: 0;
  min-height: 0;
  gap: 6px;
}
.fixed-tabs {
  display: flex;
  flex: 0 0 auto;
  gap: 4px;
  padding-right: 8px;
  border-right: 1px solid var(--border);
}
.session-tabs {
  display: flex;
  align-items: center;
  flex: 0 1 auto;
  min-width: 0;
  min-height: 0;
  gap: 4px;
  overflow-x: auto;
  overflow-y: hidden;
  overscroll-behavior: contain;
  scrollbar-width: thin;
}
.workspace-tab-item {
  display: flex;
  flex: 0 0 auto;
  max-width: 260px;
  height: var(--tab-height);
  padding-right: 6px;
  color: var(--muted);
  border-radius: var(--workspace-tab-radius);
}
.workspace-tab,
.workspace-tab-close {
  appearance: none;
  border: 0;
  border-radius: var(--workspace-tab-radius);
  background: transparent;
  color: var(--muted);
  font: inherit;
  cursor: pointer;
}
.workspace-tab {
  display: flex;
  align-items: center;
  flex: 0 0 auto;
  gap: 4px;
  min-width: 0;
  height: var(--tab-height);
  padding: var(--tab-padding-block) 12px;
  line-height: 24px;
  white-space: nowrap;
}
.workspace-tab-item .workspace-tab {
  flex: 1;
}
.workspace-tab-title {
  overflow: hidden;
  text-overflow: ellipsis;
}
.fixed-tabs > .workspace-tab:hover,
.workspace-tab-item:hover {
  background: var(--workspace-tab-hover);
  color: var(--workspace-tab-text);
}
/* 激活态优先于悬停态，颜色跟随 Naive UI 当前主题。 */
.fixed-tabs > .workspace-tab[aria-selected="true"],
.workspace-tab-item.selected {
  background: var(--workspace-tab-active);
  color: var(--workspace-tab-text);
}
.workspace-tab-item .workspace-tab,
.workspace-tab-item .workspace-tab-close {
  background: transparent;
  color: inherit;
}
.workspace-tab:focus-visible,
.workspace-tab-close:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.workspace-tab-close {
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

.workspace-tab-item .workspace-tab-close:hover {
  background: var(--workspace-tab-close-hover);
}

.workspace-tab-close .n-icon,
.workspace-tab-close svg {
  display: block;
}

.app-tabs .new-tab {
  align-self: center;
  flex: 0 0 auto;
  width: var(--tab-height);
  height: var(--tab-height);
  padding: 0;
}
.workspace-tabs-spacer {
  flex: 1 0 0;
}

.connection-count {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--muted);
  font-size: 11px;
  white-space: nowrap;
}
.app-tabs .connection-count {
  padding: 0 8px;
}
.app-tabs.tabs-vertical {
  --tab-height: 36px;
  --tab-padding-block: 6px;
  --tab-close-size: 24px;
  flex-direction: column;
  align-items: stretch;
  width: 180px;
  height: auto;
  padding: 12px 8px;
  gap: 10px;
  border-bottom: 0;
  border-right: 1px solid var(--border);
}
.tabs-vertical .workspace-tablist,
.tabs-vertical .fixed-tabs,
.tabs-vertical .session-tabs {
  flex-direction: column;
  align-items: stretch;
}
.tabs-vertical .session-tabs {
  overflow-x: hidden;
  overflow-y: auto;
}
.tabs-vertical .workspace-tab-item {
  max-width: none;
}
.tabs-vertical .workspace-tab {
  text-align: left;
}
.tabs-vertical .fixed-tabs {
  width: 100%;
  height: auto;
  padding-right: 0;
  padding-bottom: 8px;
  border-right: 0;
  border-bottom: 1px solid var(--border);
}
.tabs-vertical .session-tabs {
  width: 100%;
  flex: 0 1 auto;
  min-height: 0;
}
.tabs-vertical .connection-count {
  flex-shrink: 0;
  padding: 10px 8px 0;
}
@media (max-width: 1000px) {
  .connection-count {
    display: none;
  }
}
@media (max-width: 900px) {
  .app-tabs.tabs-vertical {
    width: 145px;
  }
}

/* 拖动使用 Pointer Events，固定页没有排序标记，也不会进入分屏命中。 */
.workspace-tab-item {
  position: relative;
  touch-action: none;
}
.workspace-tab-item.dragging {
  opacity: 0.45;
}
.workspace-tab-item.drop-before::before,
.workspace-tab-item.drop-after::after {
  content: "";
  position: absolute;
  z-index: 1;
  top: 3px;
  bottom: 3px;
  width: 2px;
  background: var(--accent);
  pointer-events: none;
}
.workspace-tab-item.drop-before::before {
  left: 0;
}
.workspace-tab-item.drop-after::after {
  right: 0;
}
.tabs-vertical .workspace-tab-item.drop-before::before,
.tabs-vertical .workspace-tab-item.drop-after::after {
  left: 3px;
  right: 3px;
  width: auto;
  height: 2px;
  bottom: auto;
}
.tabs-vertical .workspace-tab-item.drop-before::before {
  top: 0;
}
.tabs-vertical .workspace-tab-item.drop-after::after {
  top: auto;
  bottom: 0;
}
.workspace-drag-label {
  position: fixed;
  z-index: 3000;
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: var(--workspace-tab-radius);
  background: var(--surface);
  color: var(--text);
  pointer-events: none;
}
/* 插入提示固定在会话区末尾；拖动时新建按钮不响应hover或接收点击。 */
.workspace-tab-end {
  position: relative;
  align-self: center;
  flex: 0 0 auto;
  width: var(--tab-height);
  height: var(--tab-height);
}
.app-tabs.is-dragging .new-tab {
  pointer-events: none;
}
.workspace-tab-end.drop-at-end::before {
  content: "";
  position: absolute;
  left: -4px;
  top: 3px;
  bottom: 3px;
  width: 2px;
  background: var(--accent);
  pointer-events: none;
}
.tabs-vertical .workspace-tab-end {
  align-self: stretch;
  width: 100%;
}
.tabs-vertical .workspace-tab-end.drop-at-end::before {
  left: 3px;
  right: 3px;
  top: -6px;
  bottom: auto;
  width: auto;
  height: 2px;
}

/* 只有显式标记的空白区域拖动窗口，标签继续处理排序、菜单和双击动作。 */
.app-tabs.titlebar-mac {
  padding-left: 88px;
}
.app-tabs > .workspace-tabs-spacer {
  min-width: 32px;
  align-self: stretch;
}
.app-tabs.tabs-vertical {
  grid-column: 1;
  grid-row: 2;
  min-height: 0;
}
.app-tabs.tabs-vertical .new-tab {
  width: 100%;
  margin: 0;
}
</style>

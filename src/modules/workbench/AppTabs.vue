<script setup lang="ts">
import { t } from "../../i18n";
import { terminalBaseTitle } from "../terminal/title";
import ScrollFade from "../../components/ui/ScrollFade.vue";
import { CloseOutline, DocumentTextOutline } from "@vicons/ionicons5";
import { computed, nextTick, ref, watch } from "vue";
import { NIcon, useThemeVars } from "naive-ui";
import { isMac } from "../../services/commands/registry";
import AssetIcon from "../../components/ui/AssetIcon.vue";
import WindowControls from "../../components/WindowControls.vue";
import AppButton from "../../components/ui/AppButton.vue";
import { stateLabels, useWorkbenchStore, type TabBarItem } from "./store";
import { useTabDrag } from "./useTabDrag";
import { useTabScroll } from "./useTabScroll";
import { useWindowDrag } from "./useWindowDrag";
const props = defineProps<{
  tabs: TabBarItem[];
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
const workbench = useWorkbenchStore();
const windowDrag = useWindowDrag({
  enabled: () => props.position === "top" && !workbench.drag,
  onError: (message) => emit("windowError", message),
});
const root = ref<HTMLElement>();
const sessionTabs = ref<HTMLElement>();
const tabEnd = ref<HTMLElement>();
const { start: startTabDrag, insertionMarker } = useTabDrag({
  root,
  viewport: sessionTabs,
  end: tabEnd,
  position: () => props.position,
  onMoved: (id) => emit("focusTerminal", id),
});
useTabScroll(sessionTabs, () => props.position);
const dragTitle = computed(() => {
  const source = workbench.drag?.source;
  if (!source) return "";
  if (source.kind === "pane") {
    const terminal = workbench.terminals.find(
      (terminal) => terminal.terminalId === source.terminalId,
    );
    return terminal ? terminalBaseTitle(terminal) : undefined;
  }
  return props.tabs.find((tab) => tab.id === source.tabId)?.title;
});
const fixedTabs = computed(() => [
  { id: "home", title: t("common.navigation.vault"), panel: "page-home" },
  { id: "sftp", title: "SFTP", panel: "page-sftp" },
]);
function selectedElement() {
  return root.value?.querySelector<HTMLElement>(
    '[role="tab"][aria-selected="true"]',
  );
}
function activate(id: string) {
  emit("activate", id);
  if (props.tabs.some((tab) => tab.id === id && tab.kind === "terminal"))
    emit("focusTerminal", id);
}
async function onKey(event: KeyboardEvent) {
  const target = event.target as HTMLElement;
  if (target.getAttribute("role") !== "tab" || event.isComposing) return;
  const id = target.dataset.appTab!;
  const session = props.tabs.some((tab) => tab.id === id);
  if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) {
    event.preventDefault();
    if (props.tabs.some((tab) => tab.id === id && tab.kind === "terminal"))
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
    ...fixedTabs.value.map((tab) => tab.id),
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
      'is-dragging': !!workbench.drag,
      'tabs-vertical': position === 'left',
      'titlebar-mac': isMac && position === 'top',
    }"
    role="tablist"
    :aria-label="t('workbench.tabs.label')"
    :aria-orientation="position === 'left' ? 'vertical' : 'horizontal'"
    @keydown="onKey"
    :style="{
      '--app-tab-hover': themeVars.buttonColor2,
      '--app-tab-active': themeVars.buttonColor2Hover,
      '--app-tab-text': themeVars.textColor1,
      '--app-tab-close-hover': themeVars.closeColorHover,
      '--app-tab-radius': themeVars.borderRadius,
    }"
  >
    <!-- 固定页、分隔线与动态滚动区直接参与 Header 的 flex 排列。 -->
    <div class="fixed-tabs" role="presentation" @pointerdown="windowDrag.start">
      <button
        v-for="tab in fixedTabs"
        :key="tab.id"
        type="button"
        role="tab"
        class="app-tab"
        :id="'app-tab-' + tab.id"
        :data-app-tab="tab.id"
        :aria-selected="activeId === tab.id"
        :tabindex="activeId === tab.id ? 0 : -1"
        :aria-controls="tab.panel"
        @click="activate(tab.id)"
      >
        <AssetIcon
          class="fixed-tab-icon"
          :name="tab.id === 'home' ? 'vault' : 'sftp'"
          :size="16"
        />
        {{ tab.title }}
      </button>
    </div>
    <span class="tab-divider" aria-hidden="true" />
    <ScrollFade
      v-if="tabs.length"
      class="session-tabs-fade"
      :viewport="sessionTabs"
      :direction="position === 'left' ? 'vertical' : 'horizontal'"
      background="var(--surface)"
      :size="20"
      role="presentation"
    >
      <div ref="sessionTabs" class="session-tabs" role="presentation">
        <div
          v-for="tab in tabs"
          :key="tab.id"
          class="app-tab-item"
          :class="{
            selected: activeId === tab.id,
            dragging:
              workbench.drag?.source.kind === 'tab' &&
              workbench.drag.source.tabId === tab.id,
          }"
          :data-sort-tab="tab.kind === 'terminal' ? tab.id : undefined"
          @pointerdown="tab.kind === 'terminal' && startTabDrag($event, tab.id)"
          role="presentation"
          @contextmenu.prevent="
            tab.kind === 'terminal' && emit('menu', $event, tab.id)
          "
        >
          <button
            type="button"
            role="tab"
            class="app-tab"
            :id="'app-tab-' + tab.id"
            :data-app-tab="tab.id"
            :aria-selected="activeId === tab.id"
            :tabindex="activeId === tab.id ? 0 : -1"
            :aria-controls="'tab-panel-' + tab.id"
            :aria-label="
              tab.kind === 'log'
                ? tab.title
                : tab.title +
                  (tab.split
                    ? ' · ' +
                      t('workbench.terminalCount', {
                        count: tab.terminalIds.length,
                      })
                    : '') +
                  ' · ' +
                  t(stateLabels[tab.state])
            "
            :title="
              tab.kind === 'log'
                ? tab.title
                : tab.title + ' · ' + t(stateLabels[tab.state])
            "
            @click="activate(tab.id)"
            @dblclick="tab.kind === 'terminal' && emit('doubleClick', tab.id)"
          >
            <NIcon v-if="tab.kind === 'log'" :size="14" aria-hidden="true">
              <DocumentTextOutline />
            </NIcon>
            <i
              v-else
              class="status-dot"
              :class="tab.state"
              aria-hidden="true"
            />
            <span class="app-tab-title">{{ tab.title }}</span>
          </button>
          <button
            type="button"
            class="app-tab-close"
            tabindex="-1"
            :aria-label="t('workbench.actions.closeNamed', { name: tab.title })"
            :title="t('workbench.actions.closeNamed', { name: tab.title })"
            @click="emit('close', tab.id)"
          >
            <NIcon :size="position === 'top' ? 16 : 14" aria-hidden="true">
              <CloseOutline />
            </NIcon>
          </button>
        </div>
      </div>
    </ScrollFade>
    <div ref="tabEnd" class="app-tab-end" @pointerdown="windowDrag.start">
      <AppButton
        class="icon-button new-tab"
        :aria-label="t('workbench.actions.newTab')"
        :title="t('workbench.actions.newTabWithShortcut')"
        @click="emit('create')"
      >
        <AssetIcon name="add" :size="20" />
      </AppButton>
    </div>
    <!-- 所有插入槽共用一条指示线，位置由容器几何计算，不绑定单个标签。 -->
    <div
      v-if="insertionMarker"
      class="tab-insertion-marker"
      :style="insertionMarker"
      aria-hidden="true"
    />
    <div
      v-if="workbench.drag"
      class="tab-drag-label"
      :style="{
        left: workbench.drag.x + 12 + 'px',
        top: workbench.drag.y + 12 + 'px',
      }"
      aria-hidden="true"
    >
      {{ dragTitle }}
    </div>
    <div
      class="app-tabs-spacer"
      :data-tauri-drag-region="position === 'top' ? '' : undefined"
      aria-hidden="true"
    />
    <span
      :data-tauri-drag-region="position === 'top' ? '' : undefined"
      class="connection-count"
    >
      <i class="status-dot" :class="{ connected: connected > 0 }" />
      {{ t("common.count.connections", { count: connected }) }}
    </span>
    <WindowControls
      v-if="position === 'top'"
      @error="emit('windowError', $event)"
    />
  </nav>
</template>

<style scoped>
.app-tabs {
  position: relative;
  --tab-height: 32px;
  --tab-padding-block: 4px;
  --tab-close-size: 22px;
  display: flex;
  align-items: center;
  flex-shrink: 0;
  height: 54px;
  gap: 4px;
  padding: 0 12px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}
.fixed-tab-icon {
  flex-shrink: 0;
}
.fixed-tabs {
  display: flex;
  flex: 0 0 auto;
  gap: 4px;
}
.tab-divider {
  flex: 0 0 1px;
  width: 1px;
  height: 20px;
  background: var(--border);
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
/* 顶部隐藏原生滚动条，保留滚轮、触控板、键盘和拖动边缘滚动。 */
.app-tabs:not(.tabs-vertical) .session-tabs {
  scrollbar-width: none;
}
.app-tabs:not(.tabs-vertical) .session-tabs::-webkit-scrollbar {
  display: none;
}
.session-tabs-fade {
  display: flex;
  flex: 0 1 auto;
}
.app-tab-item {
  display: flex;
  flex: 0 0 auto;
  max-width: 260px;
  height: var(--tab-height);
  touch-action: none;
  padding-right: 6px;
  color: var(--muted);
  border-radius: var(--app-tab-radius);
}
.app-tab,
.app-tab-close {
  appearance: none;
  border: 0;
  border-radius: var(--app-tab-radius);
  background: transparent;
  color: var(--muted);
  font: inherit;
  cursor: pointer;
}
.app-tab {
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
.app-tab-item .app-tab {
  flex: 1;
}
.app-tab-title {
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 顶部未激活标签保留轻微主题色遮罩，只改变底色，不淡化文字与图标。 */
.app-tabs:not(.tabs-vertical)
  .fixed-tabs
  > .app-tab[aria-selected="false"]:not(:hover),
.app-tabs:not(.tabs-vertical) .app-tab-item:not(.selected):not(:hover) {
  background: color-mix(in srgb, var(--app-tab-text) 4%, transparent);
}
.fixed-tabs > .app-tab:hover,
.app-tab-item:hover {
  background: var(--app-tab-hover);
  color: var(--app-tab-text);
}
/* 激活态优先于悬停态，颜色跟随 Naive UI 当前主题。 */
.fixed-tabs > .app-tab[aria-selected="true"],
.app-tab-item.selected {
  background: var(--app-tab-active);
  color: var(--app-tab-text);
}
.app-tab-item .app-tab,
.app-tab-item .app-tab-close {
  background: transparent;
  color: inherit;
}
.app-tab:focus-visible,
.app-tab-close:focus-visible {
  outline: 2px solid var(--accent);
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

.app-tab-item .app-tab-close:hover {
  background: var(--app-tab-close-hover);
}

.app-tab-close .n-icon,
.app-tab-close svg {
  display: block;
}

.app-tabs .new-tab {
  align-self: center;
  flex: 0 0 auto;
  width: var(--tab-height);
  height: var(--tab-height);
  padding: 0;
}
.app-tabs-spacer {
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
  padding: 0;
}
.app-tabs.tabs-vertical {
  --tab-height: 36px;
  --tab-padding-block: 6px;
  --tab-close-size: 24px;
  flex-direction: column;
  align-items: stretch;
  width: 180px;
  height: auto;
  padding: 8px;
  gap: 4px;
  border-bottom: 0;
  border-right: 1px solid var(--border);
}
.tabs-vertical .fixed-tabs,
.tabs-vertical .session-tabs-fade,
.tabs-vertical .session-tabs {
  flex-direction: column;
  align-items: stretch;
}
.tabs-vertical .session-tabs {
  overflow-x: hidden;
  overflow-y: auto;
}
.tabs-vertical .app-tab-item {
  max-width: none;
}
.tabs-vertical .app-tab {
  text-align: left;
}
.tabs-vertical .fixed-tabs {
  width: 100%;
  height: auto;
}
.tabs-vertical .tab-divider {
  width: 100%;
  height: 1px;
}
.tabs-vertical .session-tabs-fade,
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
.app-tab-item.dragging {
  opacity: 0.45;
}
.tab-insertion-marker {
  position: absolute;
  z-index: 3;
  background: var(--accent);
  pointer-events: none;
}
.tab-drag-label {
  position: fixed;
  z-index: 3000;
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: var(--app-tab-radius);
  background: var(--surface);
  color: var(--text);
  pointer-events: none;
}
/* 新建按钮独立于滚动区；拖动时不响应悬停或接收点击。 */
.app-tab-end {
  align-self: center;
  flex: 0 0 auto;
  width: var(--tab-height);
  height: var(--tab-height);
}
.app-tabs.is-dragging .new-tab {
  pointer-events: none;
}
.tabs-vertical .app-tab-end {
  align-self: stretch;
  width: 100%;
}
/* 只有显式标记的空白区域拖动窗口，标签继续处理排序、菜单和双击动作。 */
.app-tabs.titlebar-mac {
  padding-left: 88px;
}
.app-tabs > .app-tabs-spacer {
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

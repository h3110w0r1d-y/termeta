<script setup lang="ts">
import type { CSSProperties } from "vue";
import ApplicationMenu from "@/components/ApplicationMenu.vue";
import WindowControls from "@/components/WindowControls.vue";
import WorkbenchSidebarToggle from "@/modules/workbench/WorkbenchSidebarToggle.vue";
import { isMac } from "@/services/commands/registry";

defineProps<{
  position: "top" | "left";
  appearance: CSSProperties;
}>();
const emit = defineEmits<{ error: [message: string] }>();
</script>

<template>
  <header
    class="workbench-header"
    :class="{ 'with-left-tabs': position === 'left', 'platform-mac': isMac }"
    :style="appearance"
    data-tauri-drag-region
  >
    <ApplicationMenu v-if="!isMac" />
    <template v-if="position === 'left'">
      <span data-tauri-drag-region>Termeta</span>
      <div class="window-drag-space" data-tauri-drag-region />
    </template>
    <slot v-else />
    <WorkbenchSidebarToggle />
    <WindowControls @error="emit('error', $event)" />
  </header>
  <!-- 左侧导航与标题栏并列；改变导航位置不会移动工作台内容和终端宿主。 -->
  <slot v-if="position === 'left'" />
</template>

<style scoped>
.workbench-header {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  min-width: 0;
  height: 48px;
  gap: 4px;
  padding: 0 6px 0 8px;
  background: var(--app-tabs-background, var(--surface));
  border-top: 1px solid transparent;
  border-bottom: 1px solid var(--app-tabs-border, var(--border));
}
.workbench-header.with-left-tabs {
  grid-column: 1 / -1;
  grid-row: 1;
  gap: 12px;
  padding-right: 12px;
  padding-left: 12px;
  font-size: 13px;
}
.workbench-header.platform-mac {
  padding-left: 88px;
}
.window-drag-space {
  flex: 1;
  align-self: stretch;
}
/* 窗口按钮与导航按钮共享状态色，保持浅色终端下的强调文字可读。 */
.workbench-header :deep(.n-button:hover),
.workbench-header :deep(.n-button:focus-visible),
.workbench-header :deep(.n-button:active) {
  --app-header-button-color: var(--app-tab-text);
}
</style>

<script setup lang="ts">
import { NIcon } from "naive-ui";
import {
  IconLayoutSidebarRightCollapse,
  IconLayoutSidebarRightExpand,
} from "@tabler/icons-vue";
import AppButton from "@/components/ui/AppButton.vue";
import { t } from "@/i18n";
import { commands } from "@/services/commands/registry";
import { useWorkbenchStore } from "./store";
const workbench = useWorkbenchStore();
</script>
<template>
  <AppButton
    v-if="workbench.activeGroup || workbench.activeLogTab"
    class="icon-button workbench-sidebar-toggle"
    :aria-label="t('workbench.sidebar.toggle')"
    :title="t('workbench.sidebar.toggle')"
    :aria-expanded="workbench.sidebarOpen"
    aria-controls="workbench-sidebar"
    @pointerdown.stop
    @keydown.stop
    @click="commands.execute('workbench.toggleSidebar')"
  >
    <NIcon :size="18" aria-hidden="true">
      <IconLayoutSidebarRightCollapse v-if="workbench.sidebarOpen" />
      <IconLayoutSidebarRightExpand v-else />
    </NIcon>
  </AppButton>
</template>

<style scoped>
.workbench-sidebar-toggle {
  flex: 0 0 36px;
  width: 36px;
  height: 36px;
  padding: 0;
}
</style>

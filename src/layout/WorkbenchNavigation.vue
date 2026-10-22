<script setup lang="ts">
import { computed } from "vue";
import { NConfigProvider } from "naive-ui";
import { useSettings } from "@/modules/settings/store";
import AppTabs from "@/modules/workbench/tabs/AppTabs.vue";
import { useWorkbenchStore } from "@/modules/workbench/store";
import WorkbenchHeader from "./WorkbenchHeader.vue";
import { useWorkbenchHeaderAppearance } from "./useWorkbenchHeaderAppearance";

const emit = defineEmits<{
  focusTerminal: [id: string];
  close: [id: string];
  doubleClick: [id: string];
  menu: [event: MouseEvent | KeyboardEvent, id: string];
  create: [];
  windowError: [message: string];
}>();
const settings = useSettings();
const workbench = useWorkbenchStore();
const connected = computed(
  () =>
    workbench.terminals.filter((terminal) => terminal.state === "connected")
      .length,
);
const { theme, themeOverrides, style } = useWorkbenchHeaderAppearance();
</script>

<template>
  <NConfigProvider abstract :theme="theme" :theme-overrides="themeOverrides">
    <WorkbenchHeader
      :position="settings.value.tabPosition"
      :appearance="style"
      @error="emit('windowError', $event)"
    >
      <AppTabs
        :tabs="workbench.tabBarItems"
        :active-id="workbench.activeId"
        :position="settings.value.tabPosition"
        :connected="connected"
        :style="style"
        @activate="workbench.activateTab($event)"
        @focus-terminal="emit('focusTerminal', $event)"
        @close="emit('close', $event)"
        @double-click="emit('doubleClick', $event)"
        @menu="(event, id) => emit('menu', event, id)"
        @create="emit('create')"
        @window-error="emit('windowError', $event)"
      />
    </WorkbenchHeader>
  </NConfigProvider>
</template>

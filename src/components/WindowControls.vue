<script setup lang="ts">
import { t } from "../i18n";
import { NIcon } from "naive-ui";
import { IconCopy, IconMinus, IconPlayerStop, IconX } from "@tabler/icons-vue";
import AppButton from "./ui/AppButton.vue";
import { isMac } from "../services/commands/registry";
import { desktop, errorMessage } from "../services/desktop/client";
import { onBeforeUnmount, onMounted, ref } from "vue";
const emit = defineEmits<{ error: [message: string] }>();
const maximized = ref(false);
let unlistenResize: (() => void) | undefined;
async function act(action: () => Promise<void>) {
  try {
    await action();
  } catch (error) {
    emit("error", errorMessage(error));
  }
}
async function syncMaximized() {
  try {
    maximized.value = await desktop.isWindowMaximized();
  } catch (error) {
    emit("error", errorMessage(error));
  }
}
async function toggleMaximize() {
  await act(async () => {
    await desktop.toggleMaximizeWindow();
    await syncMaximized();
  });
}
onMounted(async () => {
  if (isMac) return;
  await syncMaximized();
  try {
    unlistenResize = await desktop.onWindowResized(() => {
      void syncMaximized();
    });
  } catch (error) {
    emit("error", errorMessage(error));
  }
});
onBeforeUnmount(() => unlistenResize?.());
</script>
<template>
  <!-- macOS 使用原生窗口按钮；其他无边框平台由应用提供窗口控件。 -->
  <div
    v-if="!isMac"
    class="window-controls"
    :aria-label="t('window.controls.label')"
  >
    <AppButton
      class="icon-button"
      :focusable="false"
      tabindex="-1"
      :aria-label="t('window.controls.minimizeLabel')"
      :title="t('window.controls.minimize')"
      @click="act(desktop.minimizeWindow)"
    >
      <NIcon :size="16" aria-hidden="true"><IconMinus /></NIcon>
    </AppButton>
    <AppButton
      class="icon-button"
      :focusable="false"
      tabindex="-1"
      :aria-label="t('window.controls.maximizeLabel')"
      :title="t('window.controls.maximize')"
      @click="toggleMaximize"
    >
      <NIcon :size="16" aria-hidden="true">
        <IconCopy v-if="maximized" class="window-copy-icon" />
        <IconPlayerStop v-else />
      </NIcon>
    </AppButton>
    <AppButton
      class="icon-button"
      :focusable="false"
      tabindex="-1"
      :aria-label="t('window.controls.close')"
      :title="t('window.controls.close')"
      @click="act(desktop.closeWindow)"
    >
      <NIcon :size="16" aria-hidden="true"><IconX /></NIcon>
    </AppButton>
  </div>
</template>

<style scoped>
.window-controls {
  display: flex;
  flex-shrink: 0;
  gap: 2px;
}
.window-controls .icon-button {
  width: 36px;
  height: 36px;
  padding: 0;
}
:deep(.window-copy-icon) {
  transform: scaleX(-1);
}
</style>

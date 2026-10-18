<script setup lang="ts">
import { t } from "../i18n";
import { NIcon } from "naive-ui";
import { IconMinus, IconSquare, IconX } from "@tabler/icons-vue";
import ApplicationMenu from "./ApplicationMenu.vue";
import AppButton from "./ui/AppButton.vue";
import { isMac } from "../services/commands/registry";
import { desktop, errorMessage } from "../services/desktop/client";
const emit = defineEmits<{ error: [message: string] }>();
async function act(action: () => Promise<void>) {
  try {
    await action();
  } catch (error) {
    emit("error", errorMessage(error));
  }
}
</script>
<template>
  <!-- macOS 使用原生红黄绿按钮；关闭始终经过应用既有的会话与传输确认。 -->
  <div
    v-if="!isMac"
    class="window-controls"
    :aria-label="t('window.controls.label')"
  >
    <ApplicationMenu />
    <AppButton
      class="icon-button"
      :aria-label="t('window.controls.minimizeLabel')"
      :title="t('window.controls.minimize')"
      @click="act(desktop.minimizeWindow)"
    >
      <NIcon :size="16" aria-hidden="true"><IconMinus /></NIcon>
    </AppButton>
    <AppButton
      class="icon-button"
      :aria-label="t('window.controls.maximizeLabel')"
      :title="t('window.controls.maximize')"
      @click="act(desktop.toggleMaximizeWindow)"
    >
      <NIcon :size="16" aria-hidden="true"><IconSquare /></NIcon>
    </AppButton>
    <AppButton
      class="icon-button"
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
</style>

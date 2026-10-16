<script setup lang="ts">
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
  <div v-if="!isMac" class="window-controls" aria-label="窗口操作">
    <ApplicationMenu />
    <AppButton
      class="icon-button"
      aria-label="最小化窗口"
      title="最小化"
      @click="act(desktop.minimizeWindow)"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h10" /></svg>
    </AppButton>
    <AppButton
      class="icon-button"
      aria-label="最大化或还原窗口"
      title="最大化 / 还原"
      @click="act(desktop.toggleMaximizeWindow)"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <rect x="3.5" y="3.5" width="9" height="9" />
      </svg>
    </AppButton>
    <AppButton
      class="icon-button"
      aria-label="关闭窗口"
      title="关闭窗口"
      @click="act(desktop.closeWindow)"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="m4 4 8 8M12 4l-8 8" />
      </svg>
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
.window-controls svg {
  width: 16px;
  height: 16px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.2;
}
</style>

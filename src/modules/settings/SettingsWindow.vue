<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from "vue";
import {
  NConfigProvider,
  NGlobalStyle,
  NAlert,
  zhCN,
  dateZhCN,
} from "naive-ui";
import SettingsPage from "./SettingsPage.vue";
import ConfirmationDialog from "../../components/ui/ConfirmationDialog.vue";
import { useAppAppearance } from "../../layout/useAppAppearance";
import { useSettingsSync } from "./useSettingsSync";
import { desktop, errorMessage } from "../../services/desktop/client";
const { naiveTheme } = useAppAppearance();
const sync = useSettingsSync();
const ready = ref(false),
  error = ref("");
const page = ref<InstanceType<typeof SettingsPage>>();
let unlisten: (() => void) | undefined;
let closing = false;
let allowClose = false;
async function close() {
  if (closing) return;
  closing = true;
  try {
    if (page.value && !(await page.value.flush())) return;
    allowClose = true;
    await desktop.closeWindow();
  } catch (e) {
    allowClose = false;
    error.value = errorMessage(e);
  } finally {
    closing = false;
  }
}
function keydown(event: KeyboardEvent) {
  if (event.key.toLowerCase() === "w" && (event.metaKey || event.ctrlKey)) {
    if (
      (event.target as HTMLElement).closest(
        "[data-modal-open], [data-shortcut-recorder]",
      )
    )
      return;
    event.preventDefault();
    void close();
  }
}
onMounted(async () => {
  try {
    await sync.start();
    ready.value = true;
    unlisten = await desktop.onCloseRequested((event) => {
      if (allowClose) return;
      event.preventDefault();
      void close();
    });
    document.addEventListener("keydown", keydown);
  } catch (e) {
    error.value = errorMessage(e);
  }
});
onBeforeUnmount(() => {
  unlisten?.();
  document.removeEventListener("keydown", keydown);
});
</script>
<template>
  <NConfigProvider :theme="naiveTheme" :locale="zhCN" :date-locale="dateZhCN">
    <NGlobalStyle />
    <div class="settings-window">
      <NAlert v-if="error || sync.error.value" type="error">{{
        error || sync.error.value
      }}</NAlert>
      <SettingsPage v-if="ready" ref="page" />
      <ConfirmationDialog />
    </div>
  </NConfigProvider>
</template>
<style scoped>
.settings-window {
  height: 100dvh;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
</style>

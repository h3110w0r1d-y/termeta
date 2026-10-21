<script setup lang="ts">
import { NButton, NCheckbox } from "naive-ui";
import { IconArrowUp, IconHome, IconRefresh } from "@tabler/icons-vue";
import { t } from "@/i18n";
import SftpPathBar from "./SftpPathBar.vue";

defineProps<{
  path: string;
  disabled: boolean;
  showHidden: boolean;
  savingHidden: boolean;
}>();
const emit = defineEmits<{
  navigate: [path: string];
  refresh: [];
  hidden: [value: boolean];
}>();
</script>

<template>
  <div class="location-toolbar">
    <div class="location-row">
      <NButton
        quaternary
        size="small"
        :disabled="disabled || path === '/'"
        :title="t('files.actions.parent')"
        :aria-label="t('files.actions.parent')"
        @click="emit('navigate', path.replace(/\/$/, '') + '/..')"
      >
        <template #icon><IconArrowUp :size="17" aria-hidden="true" /></template>
      </NButton>
      <NButton
        quaternary
        size="small"
        :disabled="disabled"
        :title="t('files.actions.home')"
        :aria-label="t('files.actions.home')"
        @click="emit('navigate', '.')"
      >
        <template #icon><IconHome :size="17" aria-hidden="true" /></template>
      </NButton>
      <SftpPathBar
        :path="path"
        :disabled="disabled"
        @navigate="emit('navigate', $event)"
      />
      <NButton
        quaternary
        size="small"
        :disabled="disabled"
        :title="t('common.actions.refresh')"
        :aria-label="t('common.actions.refresh')"
        @click="emit('refresh')"
      >
        <template #icon><IconRefresh :size="17" aria-hidden="true" /></template>
      </NButton>
    </div>
    <div class="actions-row">
      <div class="preferences">
        <NCheckbox
          size="small"
          :checked="showHidden"
          :disabled="savingHidden"
          @update:checked="emit('hidden', $event)"
        >
          {{ t("files.actions.showHidden") }}
        </NCheckbox>
        <slot name="preferences" />
      </div>
      <div class="file-actions"><slot /></div>
    </div>
  </div>
</template>

<style scoped>
.location-toolbar {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}
.location-row {
  display: flex;
  gap: 3px;
  align-items: center;
  min-width: 0;
}
.location-row > .n-button {
  flex: none;
}
.actions-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-wrap: wrap;
}
.file-actions {
  display: flex;
  gap: 6px;
  align-items: center;
  flex-wrap: wrap;
}
.preferences {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px 12px;
}
</style>

<script setup lang="ts">
import { t } from "@/i18n";
import { NText } from "naive-ui";
import AppCheckbox from "@/components/ui/AppCheckbox.vue";
defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ "update:modelValue": [boolean] }>();
</script>
<template>
  <div class="system-detection-settings">
    <AppCheckbox
      :model-value="modelValue"
      @update:model-value="emit('update:modelValue', $event)"
    >
      {{ t("settings.detection.enabled") }}
    </AppCheckbox>
    <NText depth="3">
      {{ t("settings.detection.description") }}
    </NText>
  </div>
</template>
<style scoped>
.system-detection-settings {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
</style>

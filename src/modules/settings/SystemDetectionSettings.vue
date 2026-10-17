<script setup lang="ts">
import { t } from "../../i18n";
import { NText } from "naive-ui";
import AppCheckbox from "../../components/ui/AppCheckbox.vue";
defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ "update:modelValue": [boolean] }>();
</script>
<template>
  <div class="system-detection-settings">
    <AppCheckbox
      :model-value="modelValue"
      @update:model-value="emit('update:modelValue', $event)"
    >
      {{ t("连接 SSH 时自动识别系统") }}
    </AppCheckbox>
    <NText depth="3">
      {{
        t(
          "自动为保存的 SSH 主机选择系统图标，优先保留你手动选择的图标。关闭后保留已有结果。",
        )
      }}
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

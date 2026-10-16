<script setup lang="ts">
import { NCollapse, NCollapseItem, NInput, NText } from "naive-ui";
import AppCheckbox from "../../components/ui/AppCheckbox.vue";
import scripts from "../../../resources/ssh-os-detection.json";
defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ "update:modelValue": [boolean] }>();
const labels = {
  shell: "Shell 探测",
  posix: "普通 Shell",
  fish: "Fish Shell",
  routeros: "RouterOS",
  windows: "Windows",
};
</script>
<template>
  <div class="system-detection-settings">
    <AppCheckbox
      :model-value="modelValue"
      @update:model-value="emit('update:modelValue', $event)"
    >
      连接 SSH 时自动识别系统
    </AppCheckbox>
    <NText depth="3">
      仅检测保存的 SSH
      主机，使用独立命令通道。每台主机只尝试一次，失败或中断后不再自动重试；手动选择的图标优先。关闭功能不会清除已有结果。
    </NText>
    <NText>内置检测命令（只读）</NText>
    <NCollapse>
      <NCollapseItem
        v-for="(script, key) in scripts"
        :key="key"
        :name="key"
        :title="labels[key]"
      >
        <NInput
          :value="script"
          type="textarea"
          readonly
          :autosize="{ minRows: 2, maxRows: 18 }"
          :aria-label="labels[key] + '检测命令，只读'"
        />
      </NCollapseItem>
    </NCollapse>
  </div>
</template>
<style scoped>
.system-detection-settings {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
</style>

<script setup lang="ts">
import { NColorPicker } from "naive-ui";
import { ref } from "vue";
defineProps<{ modelValue?: string; label: string; disabled?: boolean }>();
const emit = defineEmits<{ "update:modelValue": [string] }>();
const opened = ref(false);
function update(value: string) {
  if (value.startsWith("#")) {
    emit("update:modelValue", value);
    return;
  }
  // Naive UI 可用 RGB 编辑，领域配色契约统一保存 HEX。
  const channels = value.match(/[\d.]+/g)?.map(Number);
  if (!channels || channels.length < 3) return;
  const hex = channels
    .slice(0, 3)
    .map((channel) =>
      Math.max(0, Math.min(255, Math.round(channel)))
        .toString(16)
        .padStart(2, "0"),
    )
    .join("");
  const alpha = channels[3];
  emit(
    "update:modelValue",
    "#" +
      hex +
      (alpha === undefined || alpha >= 1
        ? ""
        : Math.max(0, Math.round(alpha * 255))
            .toString(16)
            .padStart(2, "0")),
  );
}
</script>
<template>
  <NColorPicker
    v-model:show="opened"
    :data-ui-popup-open="opened ? '' : undefined"
    :value="modelValue || '#ffffff'"
    :disabled="disabled"
    :aria-label="label"
    :modes="['hex', 'rgb']"
    :show-alpha="true"
    :actions="['confirm']"
    @update:value="update"
  />
</template>

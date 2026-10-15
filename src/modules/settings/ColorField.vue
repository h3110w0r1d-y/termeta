<script setup lang="ts">
import AppColorPicker from "../../components/ui/AppColorPicker.vue";
import AppButton from "../../components/ui/AppButton.vue";
import AppLabel from "../../components/ui/AppLabel.vue";
defineProps<{ modelValue?: string; label: string; reset?: string }>();
const emit = defineEmits<{ "update:modelValue": [string] }>();
</script>
<template>
  <AppLabel
    >{{ label }}
    <div class="color-field-row">
      <AppColorPicker
        :model-value="modelValue"
        :label="label"
        @update:model-value="emit('update:modelValue', $event)"
      /><AppButton
        v-if="reset"
        title="重置单色"
        :aria-label="label + '重置单色'"
        @click="emit('update:modelValue', reset)"
        >↺</AppButton
      >
    </div></AppLabel
  >
</template>
<style scoped>
.color-field-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>

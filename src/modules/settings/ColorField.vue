<script setup lang="ts">
import AppColorPicker from "../../components/ui/AppColorPicker.vue";
import AppButton from "../../components/ui/AppButton.vue";
import { NFormItem, type FormItemRule } from "naive-ui";
import { computed } from "vue";
const props = defineProps<{
  modelValue?: string;
  label: string;
  reset?: string;
  path: string;
  optional?: boolean;
}>();
const rule = computed<FormItemRule>(() => ({
  required: !props.optional,
  trigger: ["change", "blur"],
  validator: (_, value: unknown) =>
    (props.optional && !value) ||
    (typeof value === "string" &&
      /^#[0-9a-f]{6}([0-9a-f]{2})?$/i.test(value)) ||
    new Error("请输入6或8位HEX颜色"),
}));
const emit = defineEmits<{ "update:modelValue": [string] }>();
</script>
<template>
  <NFormItem :label="label" :path="path" :rule="rule">
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
    </div></NFormItem
  >
</template>
<style scoped>
.color-field-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>

<script setup lang="ts">
import { NIcon } from "naive-ui";
import { RefreshOutline } from "@vicons/ionicons5";
import AppColorPicker from "../../components/ui/AppColorPicker.vue";
import AppButton from "../../components/ui/AppButton.vue";
import { NFormItem, type FormItemInst, type FormItemRule } from "naive-ui";
import { computed, ref } from "vue";
const props = defineProps<{
  modelValue?: string;
  label: string;
  reset?: string;
  path: string;
  optional?: boolean;
}>();
const formItem = ref<FormItemInst>();
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
function resetColor() {
  if (!props.reset) return;
  emit("update:modelValue", props.reset);
  // 重置按钮不触发取色器的 change；写入共享 model 后重验字段以清除旧错误。
  void formItem.value?.validate({ trigger: "change" }).catch(() => undefined);
}
</script>
<template>
  <NFormItem ref="formItem" :label="label" :path="path" :rule="rule">
    <div class="color-field-row">
      <div class="color-field-picker">
        <AppColorPicker
          :model-value="modelValue"
          :label="label"
          @update:model-value="emit('update:modelValue', $event)"
        />
      </div>
      <AppButton
        class="color-field-reset"
        v-if="reset"
        title="重置单色"
        :aria-label="label + '重置单色'"
        @click="resetColor"
      >
        <NIcon :size="16" aria-hidden="true"><RefreshOutline /></NIcon>
      </AppButton>
    </div>
  </NFormItem>
</template>
<style scoped>
.color-field-row {
  display: flex;
  width: 100%;
  min-width: 0;
  gap: 8px;
  align-items: center;
}
/* 用原生容器为库取色器提供确定宽度，避免绝对定位的色块无法撑开布局。 */
.color-field-picker {
  flex: 1;
  min-width: 0;
}
.color-field-reset {
  flex-shrink: 0;
}
</style>

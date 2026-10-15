<script setup lang="ts">
import { useAttrs } from "vue";
import { NInputNumber } from "naive-ui";
defineOptions({ inheritAttrs: false });
withDefaults(
  defineProps<{
    modelValue: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    readonly?: boolean;
    label?: string;
    id?: string;
  }>(),
  { disabled: undefined },
);
const emit = defineEmits<{ "update:modelValue": [value: number] }>();
const attrs = useAttrs();
</script>
<template>
  <NInputNumber
    v-bind="attrs"
    class="ui-number-field"
    :value="Number.isFinite(modelValue) ? modelValue : null"
    :min="min"
    :max="max"
    :step="step"
    :disabled="disabled"
    :readonly="readonly"
    :input-props="{
      id,
      required: attrs.required !== undefined && attrs.required !== false,
      'aria-label': label ?? (attrs['aria-label'] as string | undefined),
    }"
    @update:value="emit('update:modelValue', $event ?? Number.NaN)"
  />
</template>

<script setup lang="ts" generic="T extends string | number | null | undefined">
import { computed, useAttrs } from "vue";
import { NSelect } from "naive-ui";
defineOptions({ inheritAttrs: false });
const props = withDefaults(
  defineProps<{
    modelValue: T;
    options: { value: T; label: string; disabled?: boolean }[];
    disabled?: boolean;
    label?: string;
  }>(),
  { disabled: undefined },
);
const emit = defineEmits<{ "update:modelValue": [value: T] }>();
const attrs = useAttrs();
const menuProps = { class: "ui-select-popup", "data-ui-popup-open": "" };
// 业务值不直接传给选择器，避免空串、null 和 undefined 的空值语义冲突。
const options = computed(() =>
  props.options.map((option, index) => ({ ...option, value: index })),
);
const encoded = computed(() => {
  const index = props.options.findIndex((option) =>
    Object.is(option.value, props.modelValue),
  );
  return index < 0 ? null : index;
});
function update(value: number | string | (number | string)[] | null) {
  if (typeof value !== "number") return;
  const option = props.options[value];
  if (option && !option.disabled) emit("update:modelValue", option.value);
}
</script>
<template>
  <NSelect
    display-directive="if"
    v-bind="attrs"
    :id="undefined"
    class="ui-select-trigger"
    :value="encoded"
    :options="options"
    :disabled="disabled"
    :input-props="{
      id: attrs.id as string | undefined,
      'aria-label': label ?? (attrs['aria-label'] as string | undefined),
    }"
    :menu-props="menuProps"
    @update:value="update"
  />
</template>

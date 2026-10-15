<script setup lang="ts" generic="T extends string | number = string">
import { computed, useAttrs } from "vue";
import { NInput } from "naive-ui";
defineOptions({ inheritAttrs: false });
const props = defineProps<{
  modelValue?: T;
  value?: string | number;
  modelModifiers?: { number?: boolean; trim?: boolean };
}>();
const emit = defineEmits<{
  "update:modelValue": [value: T];
  input: [event: Event];
  change: [event: Event];
}>();
const attrs = useAttrs();
const inputProps = computed(() => {
  const rest = Object.fromEntries(
    Object.entries(attrs).filter(
      ([key]) => key !== "class" && key !== "style" && !key.startsWith("on"),
    ),
  );
  return {
    ...rest,
    onInput: (event: Event) => emit("input", event),
    onChange: (event: Event) => emit("change", event),
  };
});
function update(value: string) {
  let next: string | number = props.modelModifiers?.trim ? value.trim() : value;
  if (props.modelModifiers?.number) {
    const parsed = Number.parseFloat(String(next));
    if (!Number.isNaN(parsed)) next = parsed;
  }
  emit("update:modelValue", next as T);
}
</script>
<template>
  <NInput
    v-bind="attrs"
    class="ui-input"
    :id="undefined"
    :value="String(modelValue ?? value ?? '')"
    :type="attrs.type === 'password' ? 'password' : 'text'"
    :input-props="inputProps"
    @update:value="update"
  />
</template>

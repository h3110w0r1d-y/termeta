<script setup lang="ts">
import { computed, useAttrs } from "vue";
import { NInput } from "naive-ui";
defineOptions({ inheritAttrs: false });
defineProps<{ modelValue?: string; value?: string }>();
const emit = defineEmits<{
  "update:modelValue": [value: string];
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
</script>
<template>
  <NInput
    v-bind="attrs"
    class="ui-textarea"
    :id="undefined"
    type="textarea"
    :value="modelValue ?? value ?? ''"
    :input-props="inputProps"
    @update:value="emit('update:modelValue', $event)"
  />
</template>

<script setup lang="ts">
import { computed } from "vue";
const props = defineProps<{
  modelValue?: string;
  label: string;
  reset?: string;
}>();
const emit = defineEmits<{ "update:modelValue": [string] }>();
const rgb = computed(() => {
  const value = props.modelValue ?? "#ffffff";
  return [1, 3, 5]
    .map((i) => parseInt(value.slice(i, i + 2), 16) || 0)
    .join(", ");
});
function updateRgb(event: Event) {
  const values = (event.target as HTMLInputElement).value
    .split(",")
    .map((v) => Number(v.trim()));
  if (
    values.length !== 3 ||
    values.some((v) => !Number.isInteger(v) || v < 0 || v > 255)
  )
    return;
  emit(
    "update:modelValue",
    "#" +
      values.map((v) => v.toString(16).padStart(2, "0")).join("") +
      (props.modelValue?.slice(7) ?? ""),
  );
}
</script>
<template>
  <label
    >{{ label }}
    <div class="color-input">
      <input
        :value="modelValue?.slice(0, 7) || '#ffffff'"
        type="color"
        :aria-label="label + '取色'"
        @input="
          emit('update:modelValue', ($event.target as HTMLInputElement).value)
        "
      /><input
        :value="modelValue"
        :aria-label="label + ' HEX'"
        maxlength="9"
        @input="
          emit('update:modelValue', ($event.target as HTMLInputElement).value)
        "
      /><button
        v-if="reset"
        type="button"
        class="icon-button"
        title="重置单色"
        @click="emit('update:modelValue', reset)"
      >
        ↺
      </button>
    </div>
    <input
      :value="rgb"
      :aria-label="label + ' RGB'"
      placeholder="R, G, B（0–255）"
      @change="updateRgb"
  /></label>
</template>

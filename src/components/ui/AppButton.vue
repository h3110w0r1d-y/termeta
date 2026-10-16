<script setup lang="ts">
import { computed, useAttrs, normalizeClass } from "vue";
import { NButton } from "naive-ui";
const props = withDefaults(
  defineProps<{
    type?: "button" | "submit" | "reset";
    variant?: "default" | "primary" | "error" | "warning" | "success" | "info";
  }>(),
  { type: "button" },
);
const attrs = useAttrs();
const classes = computed(() => normalizeClass(attrs.class).split(/\s+/));
const tone = computed(
  () =>
    props.variant ??
    (classes.value.includes("primary")
      ? "primary"
      : classes.value.includes("danger")
        ? "error"
        : "default"),
);
</script>
<template>
  <NButton
    :attr-type="type"
    :type="tone"
    :quaternary="classes.includes('icon-button') || classes.includes('subtle')"
    :block="classes.includes('full')"
  >
    <slot />
  </NButton>
</template>

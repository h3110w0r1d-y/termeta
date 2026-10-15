<script setup lang="ts">
import { NScrollbar } from "naive-ui";
defineProps<{ label: string }>();
</script>
<template>
  <NScrollbar :aria-label="label" style="min-width: 0; min-height: 0">
    <slot />
  </NScrollbar>
</template>

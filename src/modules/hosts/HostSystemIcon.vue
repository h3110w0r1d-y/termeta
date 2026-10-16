<script setup lang="ts">
import { computed, useId } from "vue";
import { NIcon } from "naive-ui";
import { systemIconSvg } from "./systemIcons";
const props = withDefaults(defineProps<{ icon: string; size?: number }>(), {
  size: 24,
});
const id = useId();
const svg = computed(() => systemIconSvg(props.icon, id));
</script>
<template>
  <NIcon :size="size" aria-hidden="true" class="host-system-icon">
    <!-- 仅渲染白名单内随应用分发的SVG，不能传入远端或用户标记。 -->
    <span class="system-svg" v-html="svg" />
  </NIcon>
</template>
<style scoped>
.host-system-icon {
  flex-shrink: 0;
}
.system-svg {
  display: inline-flex;
  width: 1em;
  height: 1em;
}
.system-svg :deep(svg) {
  display: block;
  width: 100%;
  height: 100%;
}
</style>

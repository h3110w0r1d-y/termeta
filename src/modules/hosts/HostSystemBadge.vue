<script setup lang="ts">
import { computed } from "vue";
import { NAvatar } from "naive-ui";
import HostSystemIcon from "./HostSystemIcon.vue";
import { systemIconBackground } from "./systemIcons";
const props = withDefaults(defineProps<{ icon: string; size?: number }>(), {
  size: 40,
});
// 卡片与侧栏沿用同一系统色，侧栏按容器尺寸等比缩小。
const style = computed(() => ({
  backgroundColor: systemIconBackground(props.icon),
  color: "#ffffff",
  borderRadius: `${props.size * 0.3}px`,
}));
</script>
<template>
  <NAvatar
    :size="size"
    :style="style"
    class="host-system-badge"
    aria-hidden="true"
  >
    <HostSystemIcon :icon="icon" :size="size * 0.6" />
  </NAvatar>
</template>
<style scoped>
.host-system-badge {
  flex-shrink: 0;
}
/* 头像插槽为图标，取消文本基线留白，使小尺寸侧栏图标也几何居中。 */
.host-system-badge :deep(.n-avatar__text) {
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}
.host-system-badge :deep(.n-icon) {
  display: flex;
}
</style>

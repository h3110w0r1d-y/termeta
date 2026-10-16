<script setup lang="ts">
import { computed } from "vue";
import { NAvatar } from "naive-ui";
import HostSystemIcon from "./HostSystemIcon.vue";
import { systemIconBackground } from "./systemIcons";
const props = defineProps<{ icon: string }>();
// 主机卡片沿用指定素材的系统色；选择器继续使用紧凑的原始图标。
const style = computed(() => ({
  backgroundColor: systemIconBackground(props.icon),
  color: "#ffffff",
  borderRadius: "12px",
}));
</script>
<template>
  <NAvatar
    :size="40"
    :style="style"
    class="host-system-badge"
    aria-hidden="true"
  >
    <HostSystemIcon :icon="icon" :size="24" />
  </NAvatar>
</template>
<style scoped>
.host-system-badge {
  flex-shrink: 0;
}
</style>

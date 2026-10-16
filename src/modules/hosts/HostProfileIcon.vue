<script setup lang="ts">
import { HardwareChipOutline, GlobeOutline } from "@vicons/ionicons5";
import { NAvatar, NIcon } from "naive-ui";
import type { HostProfile } from "../../services/desktop/types";
import HostSystemBadge from "./HostSystemBadge.vue";
withDefaults(defineProps<{ host: HostProfile; size?: number }>(), { size: 40 });
</script>
<template>
  <NAvatar
    v-if="host.kind !== 'ssh'"
    :size="size"
    :style="{ borderRadius: `${size * 0.3}px` }"
    class="connection-profile-icon"
    aria-hidden="true"
  >
    <NIcon
      :size="size * 0.6"
      :component="host.kind === 'telnet' ? GlobeOutline : HardwareChipOutline"
    />
  </NAvatar>
  <HostSystemBadge v-else :icon="host.icon" :size="size" />
</template>
<style scoped>
.connection-profile-icon {
  flex-shrink: 0;
}
.connection-profile-icon :deep(.n-avatar__text),
.connection-profile-icon :deep(.n-icon) {
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>

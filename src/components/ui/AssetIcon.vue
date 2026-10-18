<script setup lang="ts">
import { NIcon } from "naive-ui";
import {
  IconBraces,
  IconClock,
  IconFingerprint,
  IconFolder,
  IconHome,
  IconKey,
  IconVault,
  IconPlus,
  IconServer,
  IconSwitchHorizontal,
} from "@tabler/icons-vue";

const icons = {
  add: IconPlus,
  forwarding: IconSwitchHorizontal,
  sftp: IconFolder,
  hosts: IconServer,
  vault: IconVault,
  credentials: IconKey,
  "local-terminal": IconHome,
  "host-trust": IconFingerprint,
  "session-logs": IconClock,
  snippets: IconBraces,
};
defineProps<{ name: keyof typeof icons; size?: number }>();
</script>

<template>
  <NIcon class="asset-icon" :size="size" aria-hidden="true">
    <component :is="icons[name]" />
  </NIcon>
</template>

<style scoped>
.asset-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1em;
  height: 1em;
  vertical-align: middle;
}
.asset-icon :deep(svg) {
  display: block;
  width: 100%;
  height: 100%;
}
</style>

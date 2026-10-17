<script setup lang="ts">
import { NIcon } from "naive-ui";
import {
  AddOutline,
  SwapHorizontalOutline,
  FolderOpenOutline,
  HomeOutline,
  KeyOutline,
  LockClosedOutline,
  ServerOutline,
  ShieldCheckmarkOutline,
  DocumentTextOutline,
  CodeSlashOutline,
} from "@vicons/ionicons5";

const icons = {
  forwarding: SwapHorizontalOutline,
  sftp: FolderOpenOutline,
  add: AddOutline,
  hosts: ServerOutline,
  vault: LockClosedOutline,
  credentials: KeyOutline,
  "local-terminal": HomeOutline,
  "host-trust": ShieldCheckmarkOutline,
  "session-logs": DocumentTextOutline,
  snippets: CodeSlashOutline,
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

<script setup lang="ts">
import { NIcon } from "naive-ui";
import {
  AddOutline,
  FolderOpenOutline,
  KeyOutline,
  LockClosedOutline,
  ServerOutline,
  ShieldCheckmarkOutline,
  TerminalOutline,
  TimeOutline,
} from "@vicons/ionicons5";

const icons = {
  sftp: FolderOpenOutline,
  add: AddOutline,
  hosts: ServerOutline,
  vault: LockClosedOutline,
  credentials: KeyOutline,
  "local-terminal": TerminalOutline,
  "host-trust": ShieldCheckmarkOutline,
  "connection-history": TimeOutline,
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

<script setup lang="ts">
import { NIcon } from "naive-ui";
import sftp from "../../assets/svg/sftp.svg?raw";
import add from "../../assets/svg/add.svg?raw";
import hosts from "../../assets/svg/hosts.svg?raw";
import vault from "../../assets/svg/vault.svg?raw";
import credentials from "../../assets/svg/credentials.svg?raw";
import localTerminal from "../../assets/svg/local-terminal.svg?raw";
import hostTrust from "../../assets/svg/host-trust.svg?raw";
import connectionHistory from "../../assets/svg/connection-history.svg?raw";

const icons = {
  sftp,
  add,
  hosts,
  vault,
  credentials,
  "local-terminal": localTerminal,
  "host-trust": hostTrust,
  "connection-history": connectionHistory,
};
defineProps<{ name: keyof typeof icons; size?: number }>();
</script>

<template>
  <NIcon :size="size" aria-hidden="true">
    <!-- 仅渲染随项目分发的静态 SVG，不接收外部或用户提供的标记。 -->
    <span class="asset-icon" v-html="icons[name]" />
  </NIcon>
</template>

<style scoped>
.asset-icon {
  display: inline-flex;
  width: 1em;
  height: 1em;
}
.asset-icon :deep(svg) {
  display: block;
  width: 100%;
  height: 100%;
}
</style>

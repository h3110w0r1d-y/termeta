<script setup lang="ts">
import AssetIcon from "../components/ui/AssetIcon.vue";
import type { VaultSection } from "../modules/workspace/store";
defineProps<{ section: VaultSection }>();
const icons = {
  hosts: "hosts",
  credentials: "credentials",
  "known-hosts": "host-trust",
  history: "connection-history",
} as const;
</script>
<template>
  <AssetIcon :name="icons[section]" :size="18" />
</template>

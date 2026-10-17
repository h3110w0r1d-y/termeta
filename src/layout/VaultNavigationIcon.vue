<script setup lang="ts">
import AssetIcon from "../components/ui/AssetIcon.vue";
import type { VaultSection } from "../modules/workbench/store";
defineProps<{ section: VaultSection }>();
const icons = {
  forwarding: "forwarding",
  hosts: "hosts",
  credentials: "credentials",
  "known-hosts": "host-trust",
  history: "connection-history",
} as const;
</script>
<template>
  <AssetIcon :name="icons[section]" :size="18" />
</template>

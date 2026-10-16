<script setup lang="ts">
import { NDropdown, NButton } from "naive-ui";
import { commands } from "../services/commands/registry";
</script>
<template>
  <NDropdown
    :options="[{ key: 'settings', label: '设置…' }]"
    @select="commands.execute('settings.open')"
  >
    <NButton quaternary aria-label="应用菜单">☰</NButton>
  </NDropdown>
</template>

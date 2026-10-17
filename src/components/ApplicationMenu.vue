<script setup lang="ts">
import { t } from "../i18n";
import { MenuOutline } from "@vicons/ionicons5";
import { NDropdown, NButton, NIcon } from "naive-ui";
import { commands } from "../services/commands/registry";
</script>
<template>
  <NDropdown
    :options="[{ key: 'settings', label: t('设置') }]"
    @select="commands.execute('settings.open')"
  >
    <NButton quaternary :aria-label="t('应用菜单')">
      <NIcon :size="20" aria-hidden="true"><MenuOutline /></NIcon>
    </NButton>
  </NDropdown>
</template>

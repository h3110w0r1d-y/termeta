<script setup lang="ts">
import { t } from "../i18n";
import { IconMenu2 } from "@tabler/icons-vue";
import { NDropdown, NButton, NIcon } from "naive-ui";
import { commands } from "../services/commands/registry";
</script>
<template>
  <NDropdown
    trigger="click"
    :options="[{ key: 'settings', label: t('settings.title') }]"
    @select="commands.execute('settings.open')"
  >
    <NButton
      class="application-menu-button"
      quaternary
      :focusable="false"
      tabindex="-1"
      :aria-label="t('common.navigation.appMenu')"
    >
      <NIcon :size="20" aria-hidden="true"><IconMenu2 /></NIcon>
    </NButton>
  </NDropdown>
</template>

<style scoped>
.application-menu-button {
  height: 32px;
  min-height: 32px;
  padding: 0 8px;
}
</style>

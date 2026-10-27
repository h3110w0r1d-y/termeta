<script setup lang="ts">
import { resolveTerminalScheme } from "@/theme/schemes";
import { useSettings } from "../settings/store";
import { NConfigProvider } from "naive-ui";
import { t } from "@/i18n";
import AppButton from "@/components/ui/AppButton.vue";
import type { TerminalItem } from "../workbench/types";
import { stateLabels } from "../sessions/presentation";
import { useTerminalSurface } from "./useTerminalSurface";

const props = defineProps<{
  terminal: TerminalItem;
  sftpAvailable: boolean;
  dragging: boolean;
}>();
const emit = defineEmits<{ toggleSftp: [] }>();
const settings = useSettings();
const { theme, themeOverrides, style } = useTerminalSurface(
  () => resolveTerminalScheme(props.terminal, settings.value).colors,
);
</script>

<template>
  <NConfigProvider abstract :theme="theme" :theme-overrides="themeOverrides">
    <footer class="status-bar" :style="[style, { color: 'var(--muted)' }]">
      <span>
        {{
          terminal.kind === "local"
            ? t("common.connection.local")
            : terminal.kind === "serial"
              ? t("common.connection.serial")
              : terminal.kind === "telnet"
                ? t("common.connection.telnetUnencrypted")
                : "SSH"
        }}
        · {{ t(stateLabels[terminal.state]) }}
      </span>
      <span v-if="terminal.kind === 'serial' && terminal.serialConfig">
        {{ terminal.serialConfig.baudRate }} baud ·
        {{ terminal.serialConfig.charset }}
      </span>
      <span v-else>
        {{ terminal.size.cols }} × {{ terminal.size.rows }} · UTF-8
      </span>
      <AppButton
        v-if="terminal.kind === 'ssh' && sftpAvailable"
        class="status-sftp-toggle"
        size="tiny"
        :variant="terminal.sftpOpen ? 'primary' : 'default'"
        :aria-pressed="!!terminal.sftpOpen"
        :disabled="
          dragging || (!terminal.sftpOpen && terminal.state !== 'connected')
        "
        :title="t('files.sidebar.toggle')"
        @click="emit('toggleSftp')"
      >
        SFTP
      </AppButton>
    </footer>
  </NConfigProvider>
</template>

<style scoped>
.status-bar {
  display: flex;
  flex: none;
  align-items: center;
  gap: 20px;
  padding: 8px 14px;
  border-top: 1px solid var(--border);
  background: var(--surface);
  color: var(--muted);
  font-size: 11px;
}
.status-bar span:last-child {
  margin-left: auto;
}
.status-sftp-toggle {
  margin-left: auto;
}
</style>

<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { Terminal } from "@xterm/xterm";
import { t } from "@/i18n";
import type { Scheme } from "@/theme/types";
import { applyTerminalTheme } from "./theme";

const props = defineProps<{ scheme: Scheme; active: boolean }>();
const host = ref<HTMLElement>();
let terminal: Terminal | undefined;

watch(
  [host, () => props.active],
  ([element, active]) => {
    // 折叠内容首次可见时才创建；收起保留实例，卸载时释放。
    if (!element || !active || terminal) return;
    terminal = new Terminal({
      drawBoldTextInBrightColors: false,
      cols: 65,
      rows: 9,
      fontSize: 12,
      disableStdin: true,
    });
    applyTerminalTheme(terminal, element, props.scheme);
    terminal.open(element);
    terminal.write(
      `Termeta ${t("themes.preview.label")} · 中文 / UTF-8 / 🖥\r\n\x1b[1mSSH workspace\x1b[0m  user@server ~\r\n`,
    );
    for (let i = 0; i < 16; i++) terminal.write(`\x1b[48;5;${i}m  \x1b[0m`);
    terminal.write("\r\n");
    for (let i = 16; i < 256; i++) terminal.write(`\x1b[48;5;${i}m \x1b[0m`);
    terminal.write(
      "\r\n\x1b[38;2;100;200;240mTruecolor RGB\x1b[0m  cursor ▌ / selection",
    );
  },
  { flush: "post" },
);
watch(
  () => props.scheme,
  (scheme) => {
    if (terminal && host.value)
      applyTerminalTheme(terminal, host.value, scheme);
  },
  { deep: true },
);
onBeforeUnmount(() => terminal?.dispose());
</script>

<template>
  <div ref="host" class="terminal-theme-preview" />
</template>

<style scoped>
.terminal-theme-preview {
  height: 170px;
  overflow: hidden;
  margin: 12px 0;
}
</style>

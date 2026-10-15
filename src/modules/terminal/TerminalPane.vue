<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import type { TerminalTab } from "../workspace/store";
import { terminalRegistry } from "./registry";
const props = defineProps<{ tab: TerminalTab; active: boolean }>();
const host = ref<HTMLElement>();
onMounted(() => {
  if (host.value) terminalRegistry.mount(props.tab, host.value);
  if (props.active) terminalRegistry.focus(props.tab.terminalId);
});
watch(
  () => props.active,
  (active) => {
    // 自定义工作区标签管理方向键焦点；键盘导航期间不能把焦点抢到终端。
    if (active && !document.activeElement?.closest("[role=tablist]"))
      terminalRegistry.focus(props.tab.terminalId);
  },
);
</script>
<template>
  <div
    :id="'terminal-panel-' + tab.terminalId"
    role="tabpanel"
    :aria-labelledby="'workspace-tab-' + tab.terminalId"
    v-show="active"
    class="terminal-pane"
    :aria-label="tab.title"
  >
    <div ref="host" class="terminal-host" />
  </div>
</template>

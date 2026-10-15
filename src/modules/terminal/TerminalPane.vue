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
    if (active) terminalRegistry.focus(props.tab.terminalId);
  },
);
</script>
<template>
  <div
    v-show="active"
    class="terminal-pane"
    role="tabpanel"
    :aria-label="tab.title"
  >
    <div ref="host" class="terminal-host" />
  </div>
</template>

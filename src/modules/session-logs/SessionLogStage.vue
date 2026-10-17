<script setup lang="ts">
import { useWorkbenchStore } from "../workbench/store";
import SessionLogReader from "./SessionLogReader.vue";
const workbench = useWorkbenchStore();
</script>
<template>
  <section
    v-if="workbench.activeLogTab"
    :id="'tab-panel-' + workbench.activeLogTab.id"
    :aria-labelledby="'app-tab-' + workbench.activeLogTab.id"
    role="tabpanel"
    class="session-log-stage"
  >
    <SessionLogReader
      :key="workbench.activeLogTab.log.id"
      :log="workbench.activeLogTab.log"
    />
  </section>
</template>
<style scoped>
.session-log-stage {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
</style>

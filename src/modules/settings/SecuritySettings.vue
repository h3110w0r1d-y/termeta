<script setup lang="ts">
import { NList, NListItem, NThing, NEmpty, NAlert } from "naive-ui";
import AppButton from "../../components/ui/AppButton.vue";
import { ref, onMounted } from "vue";
import { confirmAction } from "../../components/ui/confirmation";
import { desktop, errorMessage } from "../../services/desktop/client";
import type { KnownHost, HistoryEntry } from "../../services/desktop/types";
const error = ref(""),
  known = ref<KnownHost[]>([]),
  history = ref<HistoryEntry[]>([]);
onMounted(loadSecurity);
async function loadSecurity() {
  try {
    [known.value, history.value] = await Promise.all([
      desktop.listKnownHosts(),
      desktop.listHistory(),
    ]);
  } catch (e) {
    error.value = errorMessage(e);
  }
}
async function clearHistory() {
  if (await confirmAction("清空本地连接历史？不会删除主机配置。"))
    try {
      await desktop.clearHistory();
      await loadSecurity();
    } catch (e) {
      error.value = errorMessage(e);
    }
}
async function revoke(id: string) {
  if (await confirmAction("撤销信任？下次连接需要重新核验指纹。"))
    try {
      await desktop.revokeKnownHost(id);
      await loadSecurity();
    } catch (e) {
      error.value = errorMessage(e);
    }
}
</script>
<template>
  <div>
    <h3>已信任的 SSH 主机</h3>
    <NEmpty v-if="!known.length" description="尚未保存主机信任。" />
    <NList v-else bordered
      ><NListItem v-for="host in known" :key="host.id"
        ><NThing
          class="selectable-text"
          :title="host.canonicalHost + ':' + host.port"
          :description="host.keyAlgorithm + ' · ' + host.fingerprintSha256"
        /><template #suffix
          ><AppButton class="danger" @click="revoke(host.id)"
            >撤销信任</AppButton
          ></template
        ></NListItem
      ></NList
    >
    <h3>连接历史</h3>
    <AppButton :disabled="!history.length" @click="clearHistory"
      >清空历史</AppButton
    >
    <NEmpty v-if="!history.length" description="尚无连接历史。" />
    <NList v-else bordered
      ><NListItem v-for="item in history" :key="item.id"
        ><NThing
          :title="item.displayName"
          :description="
            item.endpoint +
            ' · ' +
            new Date(item.startedAt).toLocaleString() +
            ' · ' +
            (item.endReason || '进行中') +
            ' ' +
            (item.errorCode || '')
          " /></NListItem
    ></NList>
  </div>
  <NAlert v-if="error" type="error">{{ error }}</NAlert>
</template>

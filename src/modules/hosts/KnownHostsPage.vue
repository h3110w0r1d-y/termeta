<script setup lang="ts">
import {
  NList,
  NListItem,
  NThing,
  NEmpty,
  NAlert,
  NSpin,
  NH1,
  NText,
  NFlex,
} from "naive-ui";
import AppButton from "../../components/ui/AppButton.vue";
import { ref, onMounted } from "vue";
import { confirmAction } from "../../components/ui/confirmation";
import { desktop, errorMessage } from "../../services/desktop/client";
import type { KnownHost } from "../../services/desktop/types";
const error = ref("");
const known = ref<KnownHost[]>([]);
const loading = ref(true);
const busy = ref(false);
onMounted(load);
async function load() {
  loading.value = true;
  error.value = "";
  try {
    known.value = await desktop.listKnownHosts();
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    loading.value = false;
  }
}
async function revoke(id: string) {
  if (busy.value) return;
  busy.value = true;
  try {
    if (!(await confirmAction("撤销信任？下次连接需要重新核验指纹。"))) return;
    await desktop.revokeKnownHost(id);
    await load();
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <section class="home-page" aria-label="主机信任">
    <header class="page-heading">
      <div>
        <NH1>主机信任</NH1
        ><NText depth="3">Known Hosts · 已保存的 SSH 主机密钥与指纹</NText>
      </div>
      <NFlex
        ><AppButton :disabled="loading || busy" @click="load"
          >刷新</AppButton
        ></NFlex
      >
    </header>
    <NFlex vertical :size="16">
      <NAlert v-if="error" type="error">{{ error }}</NAlert>
      <NSpin :show="loading">
        <NEmpty
          v-if="!loading && !known.length"
          description="尚未保存主机信任。"
        />
        <NList v-else bordered
          ><NListItem v-for="host in known" :key="host.id"
            ><NThing
              class="selectable-text"
              :title="host.canonicalHost + ':' + host.port"
              :description="host.keyAlgorithm + ' · ' + host.fingerprintSha256"
            /><template #suffix
              ><AppButton
                class="danger"
                :disabled="loading || busy"
                @click="revoke(host.id)"
                >撤销信任</AppButton
              ></template
            ></NListItem
          ></NList
        >
      </NSpin>
    </NFlex>
  </section>
</template>

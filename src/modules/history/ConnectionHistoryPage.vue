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
import type { HistoryEntry } from "../../services/desktop/types";
const error = ref("");
const history = ref<HistoryEntry[]>([]);
const loading = ref(true);
const busy = ref(false);
onMounted(load);
async function load() {
  loading.value = true;
  error.value = "";
  try {
    history.value = await desktop.listHistory();
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    loading.value = false;
  }
}
async function clearHistory() {
  if (busy.value) return;
  busy.value = true;
  try {
    if (!(await confirmAction("清空本地连接历史？不会删除主机配置。"))) return;
    await desktop.clearHistory();
    await load();
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <section class="home-page" aria-label="连接历史">
    <header class="page-heading">
      <div>
        <NH1>连接历史</NH1>
        <NText depth="3">最近 20 次 SSH、本地终端与串口连接</NText>
      </div>
      <NFlex>
        <AppButton :disabled="loading || busy" @click="load">刷新</AppButton>
        <AppButton
          class="danger"
          :disabled="loading || busy || !history.length"
          @click="clearHistory"
        >
          清空历史
        </AppButton>
      </NFlex>
    </header>
    <NFlex vertical :size="16">
      <NAlert v-if="error" type="error">{{ error }}</NAlert>
      <NSpin :show="loading">
        <NEmpty
          v-if="!loading && !history.length"
          description="尚无连接历史。"
        />
        <NList v-else bordered>
          <NListItem v-for="item in history" :key="item.id">
            <NThing
              :title="item.displayName"
              :description="
                item.endpoint +
                ' · ' +
                new Date(item.startedAt).toLocaleString() +
                ' · ' +
                (item.endReason || '进行中') +
                ' ' +
                (item.errorCode || '')
              "
            />
          </NListItem>
        </NList>
      </NSpin>
    </NFlex>
  </section>
</template>

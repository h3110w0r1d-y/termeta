<script setup lang="ts">
import { t, formatDate } from "../../i18n";
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
    if (!(await confirmAction(t("history.confirmClear")))) return;
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
  <section class="home-page" :aria-label="t('history.title')">
    <header class="page-heading">
      <div>
        <NH1>{{ t("history.title") }}</NH1>
        <NText depth="3">{{ t("history.description") }}</NText>
      </div>
      <NFlex>
        <AppButton :disabled="loading || busy" @click="load">
          {{ t("common.actions.refresh") }}
        </AppButton>
        <AppButton
          class="danger"
          :disabled="loading || busy || !history.length"
          @click="clearHistory"
        >
          {{ t("history.actions.clear") }}
        </AppButton>
      </NFlex>
    </header>
    <NFlex vertical :size="16">
      <NAlert v-if="error" type="error">{{ error }}</NAlert>
      <NSpin :show="loading">
        <NEmpty
          v-if="!loading && !history.length"
          :description="t('history.empty')"
        />
        <NList v-else bordered>
          <NListItem v-for="item in history" :key="item.id">
            <NThing
              :title="item.displayName"
              :description="
                item.endpoint +
                ' · ' +
                formatDate(item.startedAt) +
                ' · ' +
                (item.endReason || t('history.status.inProgress')) +
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

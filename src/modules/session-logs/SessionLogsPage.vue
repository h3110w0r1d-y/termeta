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
  NPagination,
} from "naive-ui";
import AppButton from "../../components/ui/AppButton.vue";
import AppInput from "../../components/ui/AppInput.vue";
import { computed, ref, onMounted, watch } from "vue";
import { confirmAction } from "../../components/ui/confirmation";
import { desktop, errorMessage } from "../../services/desktop/client";
import type { SessionLogSummary } from "../../services/desktop/types";
import { useWorkbenchStore } from "../workbench/store";
import { sessionLogEndpoint, sessionLogProtocol } from "./presentation";
const workbench = useWorkbenchStore();
const error = ref("");
const logs = ref<SessionLogSummary[]>([]);
const query = ref("");
const page = ref(1);
const pageSize = 20;
const loading = ref(true);
const busy = ref(false);
const filtered = computed(() => {
  const needle = query.value.trim().toLocaleLowerCase();
  return logs.value.filter((log) =>
    [log.name, log.username, log.host, sessionLogProtocol(log.protocol)]
      .join(" ")
      .toLocaleLowerCase()
      .includes(needle),
  );
});
const visibleLogs = computed(() =>
  filtered.value.slice((page.value - 1) * pageSize, page.value * pageSize),
);
watch(query, () => {
  page.value = 1;
});
onMounted(load);
watch(
  () => workbench.activeId,
  (id) => {
    if (id === "home" && workbench.vaultSection === "logs") void load();
  },
);
async function load() {
  loading.value = true;
  error.value = "";
  try {
    logs.value = await desktop.listSessionLogs();
    page.value = Math.min(
      page.value,
      Math.max(1, Math.ceil(filtered.value.length / pageSize)),
    );
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    loading.value = false;
  }
}
async function removeLog(log: SessionLogSummary) {
  if (busy.value) return;
  busy.value = true;
  try {
    if (
      !(await confirmAction(t("sessionLogs.confirmDelete", { name: log.name })))
    )
      return;
    await desktop.deleteSessionLog(log.id);
    workbench.closeSessionLog(log.id);
    await load();
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
async function clearLogs() {
  if (busy.value) return;
  busy.value = true;
  try {
    if (!(await confirmAction(t("sessionLogs.confirmClear")))) return;
    await desktop.clearSessionLogs();
    workbench.closeAllSessionLogs();
    await load();
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <section class="home-page" :aria-label="t('sessionLogs.title')">
    <header class="page-heading">
      <div>
        <NH1>{{ t("sessionLogs.title") }}</NH1>
        <NText depth="3">{{ t("sessionLogs.description") }}</NText>
      </div>
      <NFlex>
        <AppButton :disabled="loading || busy" @click="load">
          {{ t("common.actions.refresh") }}
        </AppButton>
        <AppButton
          class="danger"
          :disabled="loading || busy || !logs.length"
          @click="clearLogs"
        >
          {{ t("sessionLogs.actions.clear") }}
        </AppButton>
      </NFlex>
    </header>
    <NFlex vertical :size="16">
      <NAlert v-if="error" type="error">{{ error }}</NAlert>
      <AppInput
        v-model="query"
        :placeholder="t('sessionLogs.search')"
        :aria-label="t('sessionLogs.search')"
        clearable
      />
      <NSpin :show="loading">
        <NEmpty
          v-if="!loading && !filtered.length"
          :description="
            t(logs.length ? 'sessionLogs.noMatch' : 'sessionLogs.empty')
          "
        />
        <NList v-else bordered>
          <NListItem v-for="log in visibleLogs" :key="log.id">
            <NThing
              class="selectable-text"
              :title="log.name"
              :description="
                [
                  sessionLogProtocol(log.protocol),
                  sessionLogEndpoint(log),
                  formatDate(log.closedAt),
                ]
                  .filter(Boolean)
                  .join(' · ')
              "
            />
            <template #suffix>
              <NFlex>
                <AppButton
                  :disabled="busy"
                  @click="workbench.openSessionLog(log)"
                >
                  {{ t("sessionLogs.actions.open") }}
                </AppButton>
                <AppButton
                  class="danger"
                  :disabled="loading || busy"
                  @click="removeLog(log)"
                >
                  {{ t("common.actions.delete") }}
                </AppButton>
              </NFlex>
            </template>
          </NListItem>
        </NList>
      </NSpin>
      <NPagination
        v-if="filtered.length > pageSize"
        v-model:page="page"
        :page-size="pageSize"
        :item-count="filtered.length"
        :disabled="loading || busy"
      />
    </NFlex>
  </section>
</template>

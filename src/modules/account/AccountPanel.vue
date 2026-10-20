<script setup lang="ts">
import { watch } from "vue";
import { NAlert, NButton, NSpin } from "naive-ui";
import { formatDate, t } from "@/i18n";
import { confirmAction } from "@/components/ui/confirmation";
import { desktop } from "@/services/desktop/client";
import { useAccount } from "./useAccount";
import { useOfficialSync } from "./useOfficialSync";
import BrowserAccountLogin from "./BrowserAccountLogin.vue";

const {
  account,
  providers,
  flow,
  loading,
  working,
  busy,
  blocked,
  error,
  providerError,
  loadProviders,
  perform,
  start,
  cancel,
} = useAccount({ loadProviders: true });
const {
  sync,
  error: syncError,
  refresh: refreshSync,
  invalidate: invalidateSync,
} = useOfficialSync();
watch(
  () => account.value.user?.id,
  () => {
    invalidateSync();
    void refreshSync();
  },
);
async function logout() {
  if (blocked.value || !(await confirmAction(t("account.logout.confirm"))))
    return;
  await perform(desktop.logoutAccount);
  await refreshSync();
}
async function synchronize() {
  await perform(desktop.syncOfficialVault);
  await refreshSync();
}
async function beforeClose() {
  if (busy.value) return false;
  if (flow.value) await cancel();
  return true;
}
defineExpose({ busy, beforeClose });
</script>

<template>
  <section class="account-panel" :aria-label="t('account.title')">
    <h3>{{ t("account.title") }}</h3>
    <p class="muted">{{ t("account.description") }}</p>
    <NAlert v-if="error || syncError" type="error" role="alert">
      {{ error || syncError }}
    </NAlert>
    <NSpin :show="loading">
      <template v-if="account.user">
        <strong>{{ account.user.email }}</strong>
        <NAlert type="info" :show-icon="false">
          {{ t("account.sync.description") }}
        </NAlert>
        <NAlert v-if="sync?.phase === 'needsPassword'" type="warning">
          {{ t("account.sync.mainWindowRequired") }}
        </NAlert>
        <NAlert v-if="sync?.error" type="error">{{ sync.error }}</NAlert>
        <p v-if="sync?.lastSyncedAt" class="muted">
          {{
            t("account.sync.lastSynced", {
              date: formatDate(sync.lastSyncedAt),
            })
          }}
        </p>
        <div class="account-actions">
          <NButton
            :loading="working"
            :disabled="blocked || sync?.phase !== 'ready'"
            @click="synchronize"
          >
            {{ t("account.sync.now") }}
          </NButton>
          <NButton :disabled="blocked" @click="logout">
            {{ t("account.actions.logout") }}
          </NButton>
        </div>
      </template>
      <BrowserAccountLogin
        v-else
        :providers="providers"
        :flow="flow"
        :blocked="blocked"
        :working="working"
        :provider-error="providerError"
        @start="start"
        @cancel="cancel"
        @retry="loadProviders"
      />
    </NSpin>
  </section>
</template>

<style scoped>
.account-panel {
  max-width: 760px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding-bottom: 24px;
}
.account-panel h3,
.account-panel p {
  margin: 0;
}
.account-panel :deep(.n-alert) {
  margin-top: 12px;
}
.account-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 14px;
}
</style>

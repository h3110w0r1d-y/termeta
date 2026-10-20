<script setup lang="ts">
import { t } from "@/i18n";
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
import AppButton from "@/components/ui/AppButton.vue";
import { ref, onMounted } from "vue";
import { confirmAction } from "@/components/ui/confirmation";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { KnownHost } from "@/services/desktop/types";
import { useBackupRestored } from "../backup/useBackupRestored";
const error = ref("");
const known = ref<KnownHost[]>([]);
const loading = ref(true);
const busy = ref(false);
onMounted(load);
useBackupRestored(load, (cause) => {
  error.value = errorMessage(cause);
});
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
    if (!(await confirmAction(t("knownHosts.confirmRevoke")))) return;
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
  <section class="home-page" :aria-label="t('common.navigation.knownHosts')">
    <header class="page-heading">
      <div>
        <NH1>{{ t("common.navigation.knownHosts") }}</NH1>
        <NText depth="3">{{ t("knownHosts.description") }}</NText>
      </div>
      <NFlex>
        <AppButton :disabled="loading || busy" @click="load">
          {{ t("common.actions.refresh") }}
        </AppButton>
      </NFlex>
    </header>
    <NFlex vertical :size="16">
      <NAlert v-if="error" type="error">{{ error }}</NAlert>
      <NSpin :show="loading">
        <NEmpty
          v-if="!loading && !known.length"
          :description="t('knownHosts.empty')"
        />
        <NList v-else bordered>
          <NListItem v-for="host in known" :key="host.id">
            <NThing
              class="selectable-text"
              :title="host.canonicalHost + ':' + host.port"
              :description="host.keyAlgorithm + ' · ' + host.fingerprintSha256"
            />
            <template #suffix>
              <AppButton
                class="danger"
                :disabled="loading || busy"
                @click="revoke(host.id)"
              >
                {{ t("knownHosts.actions.revoke") }}
              </AppButton>
            </template>
          </NListItem>
        </NList>
      </NSpin>
    </NFlex>
  </section>
</template>

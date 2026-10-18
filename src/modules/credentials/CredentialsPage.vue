<script setup lang="ts">
import { t } from "../../i18n";
import { NIcon } from "naive-ui";
import { IconPlus } from "@tabler/icons-vue";
import { NEmpty, NList, NListItem, NThing, NSpin, NH1, NText } from "naive-ui";
import AppInput from "../../components/ui/AppInput.vue";
import AppButton from "../../components/ui/AppButton.vue";
import { computed, onMounted, ref } from "vue";
import { confirmAction } from "../../components/ui/confirmation";
import { desktop, errorMessage } from "../../services/desktop/client";
import type { CredentialMetadata } from "../../services/desktop/types";
import { useHosts } from "../hosts/store";
import { useBackupRestored } from "../backup/useBackupRestored";
import CredentialEditor from "./CredentialEditor.vue";
const hosts = useHosts();
const items = ref<CredentialMetadata[]>([]),
  error = ref(""),
  loading = ref(true),
  busy = ref(false),
  editorOpen = ref(false),
  query = ref("");
const visible = computed(() =>
  items.value.filter((item) =>
    item.label.toLowerCase().includes(query.value.toLowerCase()),
  ),
);
function references(id: string) {
  return hosts.hosts.filter((host) =>
    host.authPlan.some((step) => step.credentialId === id),
  );
}
async function load() {
  loading.value = true;
  error.value = "";
  try {
    items.value = await desktop.listCredentials();
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    loading.value = false;
  }
}
onMounted(load);
useBackupRestored(load, (cause) => {
  error.value = errorMessage(cause);
});
function add() {
  error.value = "";
  editorOpen.value = true;
}
async function remove(item: CredentialMetadata) {
  if (
    !(await confirmAction(
      t("credentials.confirmDelete", {
        label: item.label,
      }),
    ))
  )
    return;
  busy.value = true;
  try {
    await desktop.deleteCredential(item.id);
    await load();
  } catch (e) {
    const failure = errorMessage(e);
    await load();
    error.value = failure;
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <section class="credentials-page home-page">
    <header class="page-heading">
      <div>
        <NH1>{{ t("common.navigation.credentials") }}</NH1>
        <NText depth="3">
          {{ t("credentials.description") }}
        </NText>
      </div>
      <AppButton class="primary" :disabled="busy" @click="add">
        <NIcon aria-hidden="true"><IconPlus /></NIcon>
        {{ t("credentials.actions.new") }}
      </AppButton>
    </header>
    <AppInput
      v-model="query"
      type="search"
      :aria-label="t('credentials.search.label')"
      :placeholder="t('credentials.search.placeholder')"
      class="credential-search"
    />
    <p v-if="error && !editorOpen" class="error" role="alert">{{ error }}</p>
    <NSpin
      v-if="loading"
      size="small"
      :description="t('credentials.status.loading')"
    />
    <NEmpty
      v-else-if="!visible.length"
      class="credentials-empty"
      :description="
        items.length
          ? t('credentials.search.noResults')
          : t('credentials.empty')
      "
    >
      <template #extra>
        <AppButton v-if="!items.length" @click="add">
          {{ t("credentials.actions.create") }}
        </AppButton>
      </template>
    </NEmpty>
    <NList v-else bordered>
      <NListItem v-for="item in visible" :key="item.id">
        <NThing
          :title="item.label"
          :description="
            (item.kind === 'password'
              ? t('credentials.types.password')
              : t('credentials.types.privateKey')) +
            ' · ' +
            (item.status === 'ready'
              ? t('credentials.status.ready')
              : item.status === 'pending'
                ? t('credentials.status.awaitingRecovery')
                : t('credentials.status.pendingDeletion')) +
            ' · ' +
            t('credentials.referenceCount', {
              count: references(item.id).length,
            })
          "
        >
          <span v-if="references(item.id).length">
            {{
              references(item.id)
                .map((host) => host.name)
                .join("、")
            }}
          </span>
        </NThing>
        <template #suffix>
          <div class="button-row">
            <AppButton
              class="danger"
              :disabled="busy || references(item.id).length > 0"
              :title="
                references(item.id).length
                  ? t('credentials.delete.referenceHint')
                  : t('credentials.actions.delete')
              "
              @click="remove(item)"
            >
              {{
                item.status === "deleting"
                  ? t("credentials.actions.retryDelete")
                  : t("common.actions.delete")
              }}
            </AppButton>
          </div>
        </template>
      </NListItem>
    </NList>
    <p class="credential-help muted">
      {{ t("credentials.delete.description") }}
    </p>
    <CredentialEditor
      :open="editorOpen"
      @update:open="editorOpen = $event"
      @saved="load"
    />
  </section>
</template>
<style scoped>
.credential-search {
  margin-bottom: 24px;
}
.credential-help {
  font-size: 12px;
  line-height: 1.8;
  margin-top: 20px;
}
.credentials-empty {
  padding: 60px 20px;
}
</style>

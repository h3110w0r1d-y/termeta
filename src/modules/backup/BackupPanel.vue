<script setup lang="ts">
import { t } from "@/i18n";
import { ref } from "vue";
import { IconCloudDownload, IconCloudUpload } from "@tabler/icons-vue";
import {
  NAlert,
  NButton,
  NEmpty,
  NFlex,
  NForm,
  NFormItem,
  NInput,
  NList,
  NListItem,
  NSelect,
  NThing,
  type FormInst,
  type FormItemInst,
} from "naive-ui";
import { useCloudBackup, type BackupPanelOptions } from "./useCloudBackup";

const props = withDefaults(defineProps<Partial<BackupPanelOptions>>(), {
  disabled: false,
});
const emit = defineEmits<{ "busy-change": [busy: boolean] }>();
const configForm = ref<FormInst>();
const confirmationItem = ref<FormItemInst>();
const {
  busy,
  error,
  success,
  config,
  importWarnings,
  action,
  entries,
  listed,
  hasMore,
  operationBusy,
  canReuseSecret,
  canReuseBackupPassword,
  dirty,
  configRules,
  save,
  testConnection,
  loadMore,
  execute,
  restore,
  counts,
  time,
} = useCloudBackup(props, (value) => emit("busy-change", value), {
  config: configForm,
  confirmation: confirmationItem,
});
defineExpose({ busy, save, dirty });
</script>

<template>
  <section class="backup-panel" :aria-label="t('backup.title')">
    <NFlex>
      <NButton
        type="primary"
        :disabled="operationBusy"
        :loading="busy && action === 'backup'"
        @click="execute('backup')"
      >
        <template #icon>
          <IconCloudUpload aria-hidden="true" />
        </template>
        {{ t("backup.actions.uploadVersion") }}
      </NButton>
      <NButton
        :disabled="operationBusy"
        :loading="busy && action === 'restore'"
        @click="execute('restore')"
      >
        <template #icon>
          <IconCloudDownload aria-hidden="true" />
        </template>
        {{ t("backup.actions.downloadVersion") }}
      </NButton>
    </NFlex>
    <NAlert v-if="error" type="error" role="alert">
      {{ error }}
    </NAlert>
    <NAlert v-if="success" type="success">{{ success }}</NAlert>
    <NAlert v-if="importWarnings.length" type="warning">
      <p v-for="(message, index) in importWarnings" :key="index">
        {{ message }}
      </p>
    </NAlert>
    <template v-if="listed && action === 'restore'">
      <p class="muted">{{ t("backup.download.description") }}</p>
      <NAlert type="info">
        {{ t("backup.import.mergeDescription") }}
      </NAlert>
      <NEmpty v-if="!entries.length" :description="t('backup.empty')" />
      <NList v-else bordered>
        <NListItem v-for="entry in entries" :key="entry.id">
          <NThing
            :title="
              entry.metadata
                ? `${time(entry.metadata.createdAt)} · ${entry.metadata.deviceName}`
                : t('backup.version.undecryptable')
            "
            :description="
              entry.metadata
                ? counts(entry.metadata.counts)
                : entry.error
                  ? entry.error
                  : t('backup.version.decryptError')
            "
          >
            <span v-if="entry.metadata" class="muted">
              Termeta {{ entry.metadata.appVersion }}
            </span>
          </NThing>
          <template #suffix>
            <NButton
              :disabled="operationBusy || !entry.metadata"
              @click="restore(entry)"
            >
              <template #icon>
                <IconCloudDownload aria-hidden="true" />
              </template>
              {{ t("backup.actions.restore") }}
            </NButton>
          </template>
        </NListItem>
      </NList>
      <NButton
        v-if="hasMore"
        :disabled="operationBusy"
        :loading="busy"
        @click="loadMore"
      >
        {{ t("backup.actions.loadMore") }}
      </NButton>
    </template>
    <NAlert type="info">
      {{ t("backup.description") }}
    </NAlert>
    <NForm
      ref="configForm"
      :model="config"
      :rules="configRules"
      label-placement="top"
      :disabled="operationBusy"
      class="backup-config"
      @submit.prevent="save"
    >
      <NFormItem :label="t('backup.form.provider')" path="provider">
        <NSelect
          v-model:value="config.provider"
          :options="[
            { label: 'GitHub Gist', value: 'gist' },
            { label: 'WebDAV', value: 'webdav' },
          ]"
        />
      </NFormItem>
      <NFormItem :label="t('backup.form.deviceName')" path="deviceName">
        <NInput
          v-model:value="config.deviceName"
          :maxlength="120"
          :placeholder="t('backup.form.devicePlaceholder')"
        />
      </NFormItem>
      <template v-if="config.provider === 'webdav'">
        <NFormItem :label="t('backup.form.webdavUrl')" path="endpoint">
          <NInput
            v-model:value="config.endpoint"
            placeholder="https://dav.example.com/termeta/"
          />
        </NFormItem>
        <NFormItem :label="t('common.fields.username')" path="username">
          <NInput
            v-model:value="config.username"
            :input-props="{ autocomplete: 'off' }"
          />
        </NFormItem>
      </template>
      <NFormItem
        :label="
          config.provider === 'gist'
            ? t('backup.form.githubToken')
            : t('backup.form.webdavPassword')
        "
        path="secret"
        class="backup-wide"
      >
        <NInput
          v-model:value="config.secret"
          type="password"
          show-password-on="click"
          :placeholder="
            canReuseSecret
              ? t('backup.form.savedSecretPlaceholder')
              : t('backup.form.credentialsHint')
          "
          :input-props="{ autocomplete: 'off' }"
        />
      </NFormItem>
      <NFormItem
        :label="t('backup.form.password')"
        path="backupPassword"
        class="backup-wide"
      >
        <NInput
          v-model:value="config.backupPassword"
          type="password"
          show-password-on="click"
          :placeholder="
            canReuseBackupPassword
              ? t('backup.form.savedSecretPlaceholder')
              : t('backup.form.passwordHint')
          "
          :input-props="{ autocomplete: 'new-password', maxlength: 1024 }"
        />
      </NFormItem>
      <NFormItem
        v-if="config.backupPassword"
        ref="confirmationItem"
        :label="t('backup.form.confirmPassword')"
        path="confirmation"
        class="backup-wide"
      >
        <NInput
          v-model:value="config.confirmation"
          type="password"
          :input-props="{ autocomplete: 'new-password', maxlength: 1024 }"
        />
      </NFormItem>
      <p class="muted backup-wide">
        {{ t("backup.passwordHint") }}
      </p>
    </NForm>
    <NFlex justify="end">
      <NButton
        :disabled="operationBusy"
        :loading="busy"
        @click="testConnection"
      >
        {{ t("backup.actions.test") }}
      </NButton>
      <NButton
        :disabled="operationBusy || !dirty"
        :loading="busy"
        @click="save"
      >
        {{ t("backup.actions.saveConfig") }}
      </NButton>
    </NFlex>
  </section>
</template>

<style scoped>
.backup-panel {
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding-block: 16px;
}
.backup-config {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0 16px;
}
.backup-wide {
  grid-column: 1 / -1;
}
@media (max-width: 640px) {
  .backup-config {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>

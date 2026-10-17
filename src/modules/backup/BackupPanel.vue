<script setup lang="ts">
import { t } from "../../i18n";
import { ref } from "vue";
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
  NRadioButton,
  NRadioGroup,
  NSelect,
  NThing,
  type FormInst,
  type FormItemInst,
} from "naive-ui";
import AppDialog from "../../components/ui/AppDialog.vue";
import { useCloudBackup, type BackupPanelOptions } from "./useCloudBackup";

const props = withDefaults(defineProps<Partial<BackupPanelOptions>>(), {
  initializing: false,
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
  preview,
  previewOpen,
  operationBusy,
  canReuseSecret,
  canReuseBackupPassword,
  dirty,
  configRules,
  save,
  testConnection,
  loadMore,
  execute,
  prepare,
  counts,
  time,
  apply,
} = useCloudBackup(props, (value) => emit("busy-change", value), {
  config: configForm,
  confirmation: confirmationItem,
});
defineExpose({ busy, save, dirty });
</script>

<template>
  <section class="backup-panel" :aria-label="t('backup.title')">
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
      @submit.prevent="execute"
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
        v-if="!initializing"
        :disabled="operationBusy || !dirty"
        :loading="busy"
        @click="save"
      >
        {{ t("backup.actions.saveConfig") }}
      </NButton>
    </NFlex>
    <NRadioGroup
      v-if="!initializing"
      v-model:value="action"
      :disabled="operationBusy"
      :aria-label="t('backup.form.operation')"
    >
      <NRadioButton value="backup">
        {{ t("backup.actions.uploadVersion") }}
      </NRadioButton>
      <NRadioButton value="sync">
        {{ t("backup.actions.downloadVersion") }}
      </NRadioButton>
    </NRadioGroup>
    <p class="muted">
      {{
        action === "backup"
          ? t("backup.upload.description")
          : t("backup.download.description")
      }}
    </p>
    <NButton
      type="primary"
      :disabled="operationBusy"
      :loading="busy"
      @click="execute"
    >
      {{
        action === "backup"
          ? t("backup.upload.progress")
          : t("backup.download.progress")
      }}
    </NButton>
    <NAlert v-if="error && !previewOpen" type="error" role="alert">
      {{ error }}
    </NAlert>
    <NAlert v-if="success" type="success">{{ success }}</NAlert>
    <NAlert v-if="importWarnings.length" type="warning">
      <p v-for="(message, index) in importWarnings" :key="index">
        {{ message }}
      </p>
    </NAlert>
    <template v-if="listed && action === 'sync'">
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
              @click="prepare(entry)"
            >
              {{ t("backup.actions.preview") }}
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
    <AppDialog
      :open="previewOpen"
      :title="t('backup.import.title')"
      :locked="operationBusy"
      @update:open="previewOpen = $event"
    >
      <template v-if="preview">
        <p>
          {{ time(preview.metadata.createdAt) }} ·
          {{ preview.metadata.deviceName }}
        </p>
        <p>{{ counts(preview.metadata.counts) }}</p>
        <NAlert type="info">
          {{
            initializing
              ? t("backup.import.setupDescription")
              : t("backup.import.mergeDescription")
          }}
        </NAlert>
        <NAlert v-if="error" type="error" class="backup-error">
          {{ error }}
        </NAlert>
      </template>
      <template #footer>
        <NFlex justify="end">
          <NButton :disabled="operationBusy" @click="previewOpen = false">
            {{ t("common.actions.cancel") }}
          </NButton>
          <NButton
            type="primary"
            :disabled="operationBusy || !preview"
            :loading="busy"
            @click="apply"
          >
            {{
              initializing
                ? t("backup.actions.setupAndImport")
                : t("backup.actions.confirmImport")
            }}
          </NButton>
        </NFlex>
      </template>
    </AppDialog>
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
.backup-error {
  margin-top: 16px;
}
@media (max-width: 640px) {
  .backup-config {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>

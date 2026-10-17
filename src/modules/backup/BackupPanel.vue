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
  <section class="backup-panel" :aria-label="t('备份与同步')">
    <NAlert type="info">
      {{
        t(
          "加密备份主机、分组、凭据、转发和主机信任。应用设置留在本机，请妥善保管备份密码。",
        )
      }}
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
      <NFormItem :label="t('备份服务')" path="provider">
        <NSelect
          v-model:value="config.provider"
          :options="[
            { label: 'GitHub Gist', value: 'gist' },
            { label: 'WebDAV', value: 'webdav' },
          ]"
        />
      </NFormItem>
      <NFormItem :label="t('本机设备名称')" path="deviceName">
        <NInput
          v-model:value="config.deviceName"
          :maxlength="120"
          :placeholder="t('例如：我的 MacBook')"
        />
      </NFormItem>
      <template v-if="config.provider === 'webdav'">
        <NFormItem :label="t('WebDAV HTTPS 目录地址')" path="endpoint">
          <NInput
            v-model:value="config.endpoint"
            placeholder="https://dav.example.com/termeta/"
          />
        </NFormItem>
        <NFormItem :label="t('用户名')" path="username">
          <NInput
            v-model:value="config.username"
            :input-props="{ autocomplete: 'off' }"
          />
        </NFormItem>
      </template>
      <NFormItem
        :label="
          config.provider === 'gist'
            ? t('GitHub Token（需 Gist 权限）')
            : t('WebDAV 密码 / 应用密码')
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
              ? t('已保存，留空继续使用；输入新值替换')
              : t('访问凭据仅加密保存在本机')
          "
          :input-props="{ autocomplete: 'off' }"
        />
      </NFormItem>
      <NFormItem
        :label="t('备份密码（至少12个字符）')"
        path="backupPassword"
        class="backup-wide"
      >
        <NInput
          v-model:value="config.backupPassword"
          type="password"
          show-password-on="click"
          :placeholder="
            canReuseBackupPassword
              ? t('已保存，留空继续使用；输入新值替换')
              : t('备份密码将加密保存在本机')
          "
          :input-props="{ autocomplete: 'new-password', maxlength: 1024 }"
        />
      </NFormItem>
      <NFormItem
        v-if="config.backupPassword"
        ref="confirmationItem"
        :label="t('确认备份密码')"
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
        {{
          t("备份密码会保存在本机。更改后只用于新备份，读取旧备份仍需原密码。")
        }}
      </p>
    </NForm>
    <NFlex justify="end">
      <NButton
        :disabled="operationBusy"
        :loading="busy"
        @click="testConnection"
      >
        {{ t("测试连接") }}
      </NButton>
      <NButton
        v-if="!initializing"
        :disabled="operationBusy || !dirty"
        :loading="busy"
        @click="save"
      >
        {{ t("保存云端配置") }}
      </NButton>
    </NFlex>
    <NRadioGroup
      v-if="!initializing"
      v-model:value="action"
      :disabled="operationBusy"
      :aria-label="t('备份或同步')"
    >
      <NRadioButton value="backup">{{ t("备份 · 上传新版本") }}</NRadioButton>
      <NRadioButton value="sync">{{ t("同步 · 从云端下载") }}</NRadioButton>
    </NRadioGroup>
    <p class="muted">
      {{
        action === "backup"
          ? t("每次上传保留一个新版本。")
          : t("选择备份版本，预览后确认导入。")
      }}
    </p>
    <NButton
      type="primary"
      :disabled="operationBusy"
      :loading="busy"
      @click="execute"
    >
      {{
        action === "backup" ? t("加密并上传新备份") : t("读取并解密云端版本")
      }}
    </NButton>
    <NAlert v-if="error && !previewOpen" type="error" role="alert">
      {{ error }}
    </NAlert>
    <NAlert v-if="success" type="success">{{ success }}</NAlert>
    <NAlert v-if="importWarnings.length" type="warning">
      <p v-for="(message, index) in importWarnings" :key="index">
        {{ t(message) }}
      </p>
    </NAlert>
    <template v-if="listed && action === 'sync'">
      <NEmpty
        v-if="!entries.length"
        :description="t('该云端目录 / 账户中没有备份。')"
      />
      <NList v-else bordered>
        <NListItem v-for="entry in entries" :key="entry.id">
          <NThing
            :title="
              entry.metadata
                ? `${time(entry.metadata.createdAt)} · ${entry.metadata.deviceName}`
                : t('无法解密的备份')
            "
            :description="
              entry.metadata
                ? counts(entry.metadata.counts)
                : entry.error
                  ? t(entry.error)
                  : t('密码不匹配或备份损坏。')
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
              {{ t("下载并预览") }}
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
        {{ t("加载更多备份") }}
      </NButton>
    </template>
    <AppDialog
      :open="previewOpen"
      :title="t('确认从云端同步')"
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
              ? t(
                  "将创建本机保险库并导入所选备份。保险库密码与备份密码相互独立，请分别保管。",
                )
              : t(
                  "导入会新增数据，保留本机内容和设置。重复导入会产生副本，主机信任冲突将跳过并提示。",
                )
          }}
        </NAlert>
        <NAlert v-if="error" type="error" class="backup-error">
          {{ error }}
        </NAlert>
      </template>
      <template #footer>
        <NFlex justify="end">
          <NButton :disabled="operationBusy" @click="previewOpen = false">
            {{ t("取消") }}
          </NButton>
          <NButton
            type="primary"
            :disabled="operationBusy || !preview"
            :loading="busy"
            @click="apply"
          >
            {{ initializing ? t("创建保险库并导入") : t("确认新增导入") }}
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

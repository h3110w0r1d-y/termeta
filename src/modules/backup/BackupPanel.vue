<script setup lang="ts">
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
  <section class="backup-panel" aria-label="备份与同步">
    <NAlert type="info">
      备份主机、分组、凭据、端口转发和主机信任；应用设置留在本机。数据库先压缩，再与时间和设备名称一起加密，备份密码请独立保管。
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
      <NFormItem label="备份服务" path="provider">
        <NSelect
          v-model:value="config.provider"
          :options="[
            { label: 'GitHub Gist', value: 'gist' },
            { label: 'WebDAV', value: 'webdav' },
          ]"
        />
      </NFormItem>
      <NFormItem label="本机设备名称" path="deviceName">
        <NInput
          v-model:value="config.deviceName"
          :maxlength="120"
          placeholder="例如：我的 MacBook"
        />
      </NFormItem>
      <template v-if="config.provider === 'webdav'">
        <NFormItem label="WebDAV HTTPS 目录地址" path="endpoint">
          <NInput
            v-model:value="config.endpoint"
            placeholder="https://dav.example.com/termeta/"
          />
        </NFormItem>
        <NFormItem label="用户名" path="username">
          <NInput
            v-model:value="config.username"
            :input-props="{ autocomplete: 'off' }"
          />
        </NFormItem>
      </template>
      <NFormItem
        :label="
          config.provider === 'gist'
            ? 'GitHub Token（需 Gist 权限）'
            : 'WebDAV 密码 / 应用密码'
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
              ? '已保存，留空继续使用；输入新值替换'
              : '访问凭据仅加密保存在本机'
          "
          :input-props="{ autocomplete: 'off' }"
        />
      </NFormItem>
      <NFormItem
        label="备份密码（至少12个字符）"
        path="backupPassword"
        class="backup-wide"
      >
        <NInput
          v-model:value="config.backupPassword"
          type="password"
          show-password-on="click"
          :placeholder="
            canReuseBackupPassword
              ? '已保存，留空继续使用；输入新值替换'
              : '备份密码将加密保存在本机'
          "
          :input-props="{ autocomplete: 'new-password', maxlength: 1024 }"
        />
      </NFormItem>
      <NFormItem
        v-if="config.backupPassword"
        ref="confirmationItem"
        label="确认备份密码"
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
        备份密码与云端配置一起加密保存在本机，后续备份和同步直接使用。修改密码只影响新备份，读取旧备份需配置当时使用的密码。
      </p>
    </NForm>
    <NFlex justify="end">
      <NButton
        :disabled="operationBusy"
        :loading="busy"
        @click="testConnection"
      >
        测试连接
      </NButton>
      <NButton
        v-if="!initializing"
        :disabled="operationBusy || !dirty"
        :loading="busy"
        @click="save"
      >
        保存云端配置
      </NButton>
    </NFlex>
    <NRadioGroup
      v-if="!initializing"
      v-model:value="action"
      :disabled="operationBusy"
      aria-label="备份或同步"
    >
      <NRadioButton value="backup">备份 · 上传新版本</NRadioButton>
      <NRadioButton value="sync">同步 · 从云端下载</NRadioButton>
    </NRadioGroup>
    <p class="muted">
      {{
        action === "backup"
          ? "每次上传生成独立版本，操作前保存有效配置。"
          : "使用已配置的备份密码解密版本信息，选择版本并预览确认，才会导入。"
      }}
    </p>
    <NButton
      type="primary"
      :disabled="operationBusy"
      :loading="busy"
      @click="execute"
    >
      {{ action === "backup" ? "加密并上传新备份" : "读取并解密云端版本" }}
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
      <NEmpty
        v-if="!entries.length"
        description="该云端目录 / 账户中没有备份。"
      />
      <NList v-else bordered>
        <NListItem v-for="entry in entries" :key="entry.id">
          <NThing
            :title="
              entry.metadata
                ? `${time(entry.metadata.createdAt)} · ${entry.metadata.deviceName}`
                : '无法解密的备份'
            "
            :description="
              entry.metadata
                ? counts(entry.metadata.counts)
                : entry.error || '密码不匹配或备份损坏。'
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
              下载并预览
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
        加载更多备份
      </NButton>
    </template>
    <AppDialog
      :open="previewOpen"
      title="确认从云端同步"
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
              ? "使用上方设置的本机保险库密码创建新保险库，导入所选备份，并加密保存当前云端配置。备份密码与本机密码相互独立。"
              : "将所选备份作为新增数据导入，重复导入同一备份也会新增副本。已有数据不覆盖，名称冲突时另存；同一地址的主机信任冲突会跳过并提示。应用设置保持本机配置。"
          }}
        </NAlert>
        <NAlert v-if="error" type="error" class="backup-error">
          {{ error }}
        </NAlert>
      </template>
      <template #footer>
        <NFlex justify="end">
          <NButton :disabled="operationBusy" @click="previewOpen = false">
            取消
          </NButton>
          <NButton
            type="primary"
            :disabled="operationBusy || !preview"
            :loading="busy"
            @click="apply"
          >
            {{ initializing ? "创建保险库并导入" : "确认新增导入" }}
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

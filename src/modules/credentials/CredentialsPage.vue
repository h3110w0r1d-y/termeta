<script setup lang="ts">
import { t } from "../../i18n";
import { NIcon } from "naive-ui";
import { AddOutline } from "@vicons/ionicons5";
import {
  NForm,
  NFormItem,
  type FormInst,
  type FormItemInst,
  type FormRules,
  NEmpty,
  NList,
  NListItem,
  NThing,
  NSpin,
  NFlex,
  NH1,
  NText,
} from "naive-ui";
import AppSelect from "../../components/ui/AppSelect.vue";
import AppInput from "../../components/ui/AppInput.vue";
import { requiredText, validateForm } from "../../services/forms/rules";
import AppButton from "../../components/ui/AppButton.vue";
import { computed, onMounted, onUnmounted, ref, watch, reactive } from "vue";
import { confirmAction } from "../../components/ui/confirmation";
import AppDialog from "../../components/ui/AppDialog.vue";
import { desktop, errorMessage } from "../../services/desktop/client";
import type { CredentialMetadata } from "../../services/desktop/types";
import { useHosts } from "../hosts/store";
import { useBackupRestored } from "../backup/useBackupRestored";
const hosts = useHosts();
const items = ref<CredentialMetadata[]>([]),
  error = ref(""),
  loading = ref(true),
  busy = ref(false),
  editorOpen = ref(false),
  query = ref(""),
  label = ref(""),
  secret = ref(""),
  passphrase = ref(""),
  keyFileRef = ref(""),
  keyName = ref(""),
  kind = ref<"password" | "privateKey">("password");
watch(kind, clearSecrets, { flush: "sync" });
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
onUnmounted(() => {
  clearSecrets();
});
const form = ref<FormInst>();
const keyFileItem = ref<FormItemInst>();
const formModel = reactive({ label, kind, secret, keyFileRef, passphrase });
const rules = computed<FormRules>(() => ({
  label: requiredText(t("名称")),
  kind: requiredText(t("类型")),
  secret:
    kind.value === "password"
      ? { required: true, message: t("请输入密码"), trigger: ["input", "blur"] }
      : [],
  keyFileRef: kind.value === "privateKey" ? requiredText(t("私钥文件")) : [],
}));
function clearSecrets() {
  secret.value = "";
  passphrase.value = "";
  keyFileRef.value = "";
  keyName.value = "";
}
async function selectKey() {
  if (busy.value) return;
  try {
    const file = await desktop.selectKey();
    if (file) {
      keyFileRef.value = file.keyFileRef;
      keyName.value = file.name;
      // 系统文件选择没有表单控件的 change 事件，导入后主动重验该字段。
      await keyFileItem.value
        ?.validate({ trigger: "change" })
        .catch(() => undefined);
    }
  } catch (e) {
    error.value = errorMessage(e);
  }
}
function closeEditor(open: boolean) {
  editorOpen.value = open;
  if (!open) clearSecrets();
}
function add() {
  label.value = "";
  clearSecrets();
  kind.value = "password";
  error.value = "";
  editorOpen.value = true;
}
async function save() {
  if (busy.value || !(await validateForm(form.value))) return;
  busy.value = true;
  error.value = "";
  try {
    await desktop.saveCredential({
      kind: kind.value,
      label: label.value.trim(),
      ...(kind.value === "password"
        ? { password: secret.value }
        : {
            keyFileRef: keyFileRef.value,
            passphrase: passphrase.value || undefined,
          }),
    });
    closeEditor(false);
    await load();
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    clearSecrets();
    busy.value = false;
  }
}
async function remove(item: CredentialMetadata) {
  if (
    !(await confirmAction(
      t("删除凭据“{label}”？此操作会从本地加密保险库移除秘密，不能撤销。", {
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
        <NH1>{{ t("凭据") }}</NH1>
        <NText depth="3">
          {{ t("集中管理密码和私钥，在多台主机之间安全复用。") }}
        </NText>
      </div>
      <AppButton class="primary" :disabled="busy" @click="add">
        <NIcon :size="16" aria-hidden="true"><AddOutline /></NIcon>
        {{ t("新建凭据") }}
      </AppButton>
    </header>
    <AppInput
      v-model="query"
      type="search"
      :aria-label="t('搜索凭据')"
      :placeholder="t('搜索凭据名称…')"
      class="credential-search"
    />
    <p v-if="error && !editorOpen" class="error" role="alert">{{ error }}</p>
    <NSpin v-if="loading" size="small" :description="t('正在读取凭据列表…')" />
    <NEmpty
      v-else-if="!visible.length"
      class="credentials-empty"
      :description="
        items.length
          ? t('没有匹配的凭据，尝试其他关键词。')
          : t('还没有保存凭据。创建后可在主机认证设置中使用。')
      "
    >
      <template #extra>
        <AppButton v-if="!items.length" @click="add">
          {{ t("创建凭据") }}
        </AppButton>
      </template>
    </NEmpty>
    <NList v-else bordered>
      <NListItem v-for="item in visible" :key="item.id">
        <NThing
          :title="item.label"
          :description="
            (item.kind === 'password' ? t('密码') : t('私钥')) +
            ' · ' +
            (item.status === 'ready'
              ? t('可用')
              : item.status === 'pending'
                ? t('等待恢复')
                : t('待完成删除')) +
            ' · ' +
            t('{count} 台主机引用', { count: references(item.id).length })
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
                  ? t('请先在主机设置中解除引用')
                  : t('删除凭据')
              "
              @click="remove(item)"
            >
              {{ item.status === "deleting" ? t("重试删除") : t("删除") }}
            </AppButton>
          </div>
        </template>
      </NListItem>
    </NList>
    <p class="credential-help muted">
      {{ t("删除凭据前，请先在主机设置中解除引用。") }}
    </p>
    <AppDialog
      :open="editorOpen"
      :title="t('新建凭据')"
      :description="t('加密保存到本地保险库，供主机认证时使用。')"
      :locked="busy"
      @update:open="closeEditor"
    >
      <NForm
        ref="form"
        :model="formModel"
        :rules="rules"
        label-placement="top"
        novalidate
        id="credential-editor-form"
        class="form-grid"
        @submit.prevent="save"
      >
        <NFormItem class="span-2" :label="t('名称')" path="label">
          <AppInput
            v-model="formModel.label"
            :disabled="busy"
            maxlength="120"
            :placeholder="t('例如：生产环境 SSH 密码')"
          />
        </NFormItem>
        <NFormItem class="span-2" :label="t('类型')" path="kind">
          <AppSelect
            v-model="formModel.kind"
            :disabled="busy"
            :options="[
              { value: 'password', label: t('SSH 密码') },
              { value: 'privateKey', label: t('SSH 私钥') },
            ]"
          />
        </NFormItem>
        <NFormItem
          v-if="kind === 'password'"
          class="span-2"
          :label="t('密码')"
          path="secret"
        >
          <AppInput
            v-model="formModel.secret"
            type="password"
            autocomplete="new-password"
            :disabled="busy"
          />
        </NFormItem>
        <template v-else>
          <NFormItem
            ref="keyFileItem"
            class="span-2"
            :label="t('私钥文件')"
            path="keyFileRef"
          >
            <AppButton :disabled="busy" @click="selectKey">
              {{ t("选择并导入 OpenSSH 私钥") }}
            </AppButton>
            <span class="muted">
              {{ keyName || t("选择的私钥将加密保存在保险库中。") }}
            </span>
          </NFormItem>
          <NFormItem
            class="span-2"
            :label="t('私钥解密密码（可选）')"
            path="passphrase"
          >
            <AppInput
              v-model="formModel.passphrase"
              type="password"
              autocomplete="new-password"
              :disabled="busy"
              :placeholder="t('留空时如需解密，将在连接时询问')"
            />
          </NFormItem>
        </template>
        <p v-if="error" class="error span-2" role="alert">{{ error }}</p>
      </NForm>
      <template #footer>
        <NFlex justify="end">
          <AppButton type="button" :disabled="busy" @click="closeEditor(false)">
            {{ t("取消") }}
          </AppButton>
          <AppButton
            type="submit"
            form="credential-editor-form"
            class="primary"
            :disabled="busy"
          >
            {{ busy ? t("保存中…") : t("保存凭据") }}
          </AppButton>
        </NFlex>
      </template>
    </AppDialog>
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

<script setup lang="ts">
import { NIcon } from "naive-ui";
import { AddOutline } from "@vicons/ionicons5";
import {
  NForm,
  NFormItem,
  type FormInst,
  type FormItemInst,
  type FormRules,
  NAlert,
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
onUnmounted(() => {
  clearSecrets();
});
const form = ref<FormInst>();
const keyFileItem = ref<FormItemInst>();
const formModel = reactive({ label, kind, secret, keyFileRef, passphrase });
const rules = computed<FormRules>(() => ({
  label: requiredText("名称"),
  kind: requiredText("类型"),
  secret:
    kind.value === "password"
      ? { required: true, message: "请输入密码", trigger: ["input", "blur"] }
      : [],
  keyFileRef: kind.value === "privateKey" ? requiredText("私钥文件") : [],
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
      `删除凭据“${item.label}”？此操作会从本地加密保险库移除秘密，不能撤销。`,
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
        <NH1>凭据</NH1>
        <NText depth="3">集中管理密码和私钥，在多台主机之间安全复用。</NText>
      </div>
      <AppButton class="primary" :disabled="busy" @click="add">
        <NIcon :size="16" aria-hidden="true"><AddOutline /></NIcon>
        新建凭据
      </AppButton>
    </header>
    <NAlert type="info" title="由本地加密保险库保护" class="vault-notice">
      这里只显示名称和使用情况，不读取或展示已保存的秘密。私钥在创建凭据时导入，解密密码可选。
    </NAlert>
    <AppInput
      v-model="query"
      type="search"
      aria-label="搜索凭据"
      placeholder="搜索凭据名称…"
      class="credential-search"
    />
    <p v-if="error && !editorOpen" class="error" role="alert">{{ error }}</p>
    <NSpin v-if="loading" size="small" description="正在读取凭据列表…" />
    <NEmpty
      v-else-if="!visible.length"
      class="credentials-empty"
      :description="
        items.length
          ? '没有匹配的凭据，尝试其他关键词。'
          : '还没有保存凭据。创建后可在主机认证设置中使用。'
      "
    >
      <template #extra>
        <AppButton v-if="!items.length" @click="add">创建凭据</AppButton>
      </template>
    </NEmpty>
    <NList v-else bordered>
      <NListItem v-for="item in visible" :key="item.id">
        <NThing
          :title="item.label"
          :description="
            (item.kind === 'password' ? '密码' : '私钥') +
            ' · ' +
            (item.status === 'ready'
              ? '可用'
              : item.status === 'pending'
                ? '等待恢复'
                : '待完成删除') +
            ' · ' +
            references(item.id).length +
            ' 台主机引用'
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
                  ? '请先在主机设置中解除引用'
                  : '删除凭据'
              "
              @click="remove(item)"
            >
              {{ item.status === "deleting" ? "重试删除" : "删除" }}
            </AppButton>
          </div>
        </template>
      </NListItem>
    </NList>
    <p class="credential-help muted">
      正在被主机引用的凭据需先解除引用才能删除。钥匙串仅保存保险库主密钥；主机配置和凭据统一加密保存在本地数据库。
    </p>
    <AppDialog
      :open="editorOpen"
      title="新建凭据"
      description="加密保存到本地保险库，供主机认证时使用。"
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
        <NFormItem class="span-2" label="名称" path="label">
          <AppInput
            v-model="formModel.label"
            :disabled="busy"
            maxlength="120"
            placeholder="例如：生产环境 SSH 密码"
          />
        </NFormItem>
        <NFormItem class="span-2" label="类型" path="kind">
          <AppSelect
            v-model="formModel.kind"
            :disabled="busy"
            :options="[
              { value: 'password', label: 'SSH 密码' },
              { value: 'privateKey', label: 'SSH 私钥' },
            ]"
          />
        </NFormItem>
        <NFormItem
          v-if="kind === 'password'"
          class="span-2"
          label="密码"
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
            label="私钥文件"
            path="keyFileRef"
          >
            <AppButton :disabled="busy" @click="selectKey">
              选择并导入 OpenSSH 私钥
            </AppButton>
            <span class="muted">
              {{
                keyName ||
                "通过系统文件选择器授权读取，保存后由本地保险库加密保护私钥。"
              }}
            </span>
          </NFormItem>
          <NFormItem
            class="span-2"
            label="私钥解密密码（可选）"
            path="passphrase"
          >
            <AppInput
              v-model="formModel.passphrase"
              type="password"
              autocomplete="new-password"
              :disabled="busy"
              placeholder="留空时如需解密，将在连接时询问"
            />
          </NFormItem>
        </template>
        <p v-if="error" class="error span-2" role="alert">{{ error }}</p>
      </NForm>
      <template #footer>
        <NFlex justify="end">
          <AppButton type="button" :disabled="busy" @click="closeEditor(false)">
            取消
          </AppButton>
          <AppButton
            type="submit"
            form="credential-editor-form"
            class="primary"
            :disabled="busy"
          >
            {{ busy ? "保存中…" : "保存凭据" }}
          </AppButton>
        </NFlex>
      </template>
    </AppDialog>
  </section>
</template>
<style scoped>
.vault-notice,
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

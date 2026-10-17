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
  label: requiredText("common.fields.name"),
  kind: requiredText("common.fields.type"),
  secret:
    kind.value === "password"
      ? {
          required: true,
          message: t("credentials.validation.passwordRequired"),
          trigger: ["input", "blur"],
        }
      : [],
  keyFileRef:
    kind.value === "privateKey"
      ? requiredText("credentials.form.privateKeyFile")
      : [],
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
        <NIcon :size="16" aria-hidden="true"><AddOutline /></NIcon>
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
    <AppDialog
      :open="editorOpen"
      :title="t('credentials.actions.new')"
      :description="t('credentials.form.description')"
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
        <NFormItem class="span-2" :label="t('common.fields.name')" path="label">
          <AppInput
            v-model="formModel.label"
            :disabled="busy"
            maxlength="120"
            :placeholder="t('credentials.form.namePlaceholder')"
          />
        </NFormItem>
        <NFormItem class="span-2" :label="t('common.fields.type')" path="kind">
          <AppSelect
            v-model="formModel.kind"
            :disabled="busy"
            :options="[
              { value: 'password', label: t('credentials.form.sshPassword') },
              {
                value: 'privateKey',
                label: t('credentials.form.sshPrivateKey'),
              },
            ]"
          />
        </NFormItem>
        <NFormItem
          v-if="kind === 'password'"
          class="span-2"
          :label="t('credentials.types.password')"
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
            :label="t('credentials.form.privateKeyFile')"
            path="keyFileRef"
          >
            <AppButton :disabled="busy" @click="selectKey">
              {{ t("credentials.actions.importKey") }}
            </AppButton>
            <span class="muted">
              {{ keyName || t("credentials.form.keyHint") }}
            </span>
          </NFormItem>
          <NFormItem
            class="span-2"
            :label="t('credentials.form.passphrase')"
            path="passphrase"
          >
            <AppInput
              v-model="formModel.passphrase"
              type="password"
              autocomplete="new-password"
              :disabled="busy"
              :placeholder="t('credentials.form.passphraseHint')"
            />
          </NFormItem>
        </template>
        <p v-if="error" class="error span-2" role="alert">{{ error }}</p>
      </NForm>
      <template #footer>
        <NFlex justify="end">
          <AppButton type="button" :disabled="busy" @click="closeEditor(false)">
            {{ t("common.actions.cancel") }}
          </AppButton>
          <AppButton
            type="submit"
            form="credential-editor-form"
            class="primary"
            :disabled="busy"
          >
            {{
              busy ? t("common.status.saving") : t("credentials.actions.save")
            }}
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

<script setup lang="ts">
import { computed, onBeforeUnmount, reactive, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NCollapse,
  NCollapseItem,
  NDivider,
  NForm,
  NFormItem,
  NInput,
  NSpin,
  NTag,
  type FormInst,
  type FormItemInst,
  type FormRules,
} from "naive-ui";
import { IconBrandGithub, IconBrandGoogle } from "@tabler/icons-vue";
import { formatDate, t } from "../../i18n";
import AppDialog from "../../components/ui/AppDialog.vue";
import { confirmAction } from "../../components/ui/confirmation";
import { validateForm } from "../../services/forms/rules";
import { desktop } from "../../services/desktop/client";
import type { AccountProviderId } from "../../services/desktop/types";
import { useAccount } from "./useAccount";
import AccountOAuthConfirmation from "./AccountOAuthConfirmation.vue";

const {
  account,
  providers,
  identities,
  flow,
  loading,
  working,
  busy,
  blocked,
  error,
  providerError,
  identityError,
  notice,
  loadProviders,
  perform,
  start,
  cancel,
} = useAccount();
const open = ref(false);
const registering = ref(false);
const form = ref<FormInst>();
const confirmationItem = ref<FormItemInst>();
const passwordItem = ref<FormItemInst>();
const model = reactive({ email: "", password: "", confirmation: "" });
const rules = computed<FormRules>(() => ({
  email: {
    required: true,
    trigger: ["input", "blur", "change"],
    validator: (_, value: string) =>
      (value.trim().length <= 254 &&
        /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value.trim())) ||
      new Error(t("account.validation.email")),
  },
  password: {
    required: true,
    trigger: ["input", "blur", "change"],
    validator: (_, value: string) =>
      ([...value].length >= 12 && [...value].length <= 256) ||
      new Error(t("account.validation.password")),
  },
  confirmation: {
    required: true,
    trigger: ["input", "blur", "change"],
    validator: (_, value: string) =>
      (value.length > 0 && value === model.password) ||
      new Error(t("account.validation.confirmation")),
  },
}));
watch(
  () => account.value.user?.id,
  (id) => {
    if (id) {
      open.value = false;
      clearPassword();
    }
  },
);
watch(
  () => model.password,
  () => {
    if (registering.value && model.confirmation)
      void confirmationItem.value
        ?.validate({ trigger: "input" })
        .catch(() => undefined);
  },
);
function clearPassword() {
  model.password = "";
  model.confirmation = "";
}
function begin(register: boolean) {
  registering.value = register;
  clearPassword();
  error.value = "";
  open.value = true;
}
async function submit() {
  if (blocked.value || !(await validateForm(form.value))) return;
  try {
    const completed = await perform(() =>
      registering.value
        ? desktop.registerAccount(model.email.trim(), model.password)
        : desktop.loginAccount(model.email.trim(), model.password),
    );
    if (completed) open.value = false;
  } finally {
    // 无论认证成功或失败，提交结束即清除秘密；留在表单时公开重验空值。
    clearPassword();
    if (open.value) {
      void passwordItem.value
        ?.validate({ trigger: "change" })
        .catch(() => undefined);
      if (registering.value)
        void confirmationItem.value
          ?.validate({ trigger: "change" })
          .catch(() => undefined);
    }
  }
}
const availableProviders = computed(() =>
  providers.value.filter(
    (provider) =>
      !account.value.user ||
      !identities.value.some((identity) => identity.provider === provider.id),
  ),
);
const refresh = () => perform(desktop.refreshAccount);
const reload = () => perform(desktop.getAccountStatus);
function canUnlink(provider: AccountProviderId) {
  if (providerError.value || identityError.value) return false;
  return (
    !!account.value.user?.hasPassword ||
    identities.value.some(
      (identity) =>
        identity.provider !== provider &&
        providers.value.some(
          (configured) => configured.id === identity.provider,
        ),
    )
  );
}
const hasProtectedIdentity = computed(() =>
  identities.value.some((identity) => !canUnlink(identity.provider)),
);
async function unlink(provider: AccountProviderId) {
  if (blocked.value || !canUnlink(provider)) return;
  if (await confirmAction(t("account.unlink.confirm", { provider })))
    await perform(() => desktop.unlinkAccountIdentity(provider));
}
async function logout() {
  if (blocked.value) return;
  if (await confirmAction(t("account.logout.confirm")))
    await perform(() => desktop.logoutAccount());
}
async function beforeClose() {
  if (busy.value) return false;
  if (flow.value) await cancel();
  return true;
}
defineExpose({ busy, beforeClose });
onBeforeUnmount(clearPassword);
</script>

<template>
  <section class="account-panel" :aria-label="t('account.title')">
    <h3>{{ t("account.title") }}</h3>
    <p class="muted">{{ t("account.description") }}</p>
    <NAlert type="info" :show-icon="false">
      {{ t("account.encryptionHint") }}
    </NAlert>
    <NAlert v-if="error" type="error" role="alert">{{ error }}</NAlert>
    <NAlert v-if="notice" type="success" role="status">
      {{
        t(
          notice === "completed"
            ? "account.oauth.completed"
            : "account.oauth.cancelled",
        )
      }}
    </NAlert>
    <NSpin :show="loading">
      <template v-if="account.user">
        <div class="account-summary">
          <strong>{{ account.user.email }}</strong>
          <NTag v-if="account.user.isAdmin" size="small">
            {{ t("account.admin") }}
          </NTag>
        </div>
        <p class="muted">
          {{
            t("account.createdAt", { date: formatDate(account.user.createdAt) })
          }}
        </p>
        <p v-if="account.expiresAt" class="muted">
          {{ t("account.expiresAt", { date: formatDate(account.expiresAt) }) }}
        </p>
        <div class="account-actions">
          <NButton :disabled="blocked" :loading="working" @click="refresh">
            {{ t("account.actions.refresh") }}
          </NButton>
          <NButton :disabled="blocked" @click="logout">
            {{ t("account.actions.logout") }}
          </NButton>
        </div>
        <NDivider />
        <h3>{{ t("account.identities.title") }}</h3>
        <p class="muted">{{ t("account.identities.description") }}</p>
        <p v-if="account.user.hasPassword">
          {{ t("account.identities.password") }}
        </p>
        <NAlert v-if="identityError" type="error">{{ identityError }}</NAlert>
        <div
          v-for="identity in identities"
          :key="identity.provider"
          class="identity-row"
        >
          <span>
            {{ identity.provider === "github" ? "GitHub" : "Google" }}
          </span>
          <NButton
            size="small"
            :disabled="blocked || !canUnlink(identity.provider)"
            @click="unlink(identity.provider)"
          >
            {{ t("account.actions.unlink") }}
          </NButton>
        </div>
        <p v-if="hasProtectedIdentity" class="muted">
          {{ t("account.identities.lastMethod") }}
        </p>
      </template>
      <template v-else>
        <p>{{ t("account.signedOut") }}</p>
        <div class="account-actions">
          <NButton type="primary" :disabled="blocked" @click="begin(false)">
            {{ t("account.actions.login") }}
          </NButton>
          <NButton :disabled="blocked" @click="begin(true)">
            {{ t("account.actions.register") }}
          </NButton>
          <NButton :disabled="blocked" @click="reload">
            {{ t("account.actions.retry") }}
          </NButton>
          <NButton v-if="error" :disabled="blocked" @click="logout">
            {{ t("account.actions.clearLogin") }}
          </NButton>
        </div>
      </template>
    </NSpin>
    <NDivider />
    <NAlert v-if="providerError" type="warning">{{ providerError }}</NAlert>
    <div class="account-actions">
      <NButton
        v-for="provider in availableProviders"
        :key="provider.id"
        :disabled="blocked"
        @click="start(provider.id, !!account.user)"
      >
        <template #icon>
          <IconBrandGithub
            v-if="provider.id === 'github'"
            :size="18"
            aria-hidden="true"
          />
          <IconBrandGoogle v-else :size="18" aria-hidden="true" />
        </template>
        {{
          t(
            account.user
              ? "account.actions.linkProvider"
              : "account.actions.loginProvider",
            { provider: provider.name },
          )
        }}
      </NButton>
      <NButton
        v-if="providerError"
        :disabled="blocked"
        @click="perform(loadProviders)"
      >
        {{ t("account.actions.retryProviders") }}
      </NButton>
    </div>
    <NAlert v-if="flow" type="info" role="status">
      <p>
        {{
          t("account.oauth.waiting", {
            provider: flow.provider === "github" ? "GitHub" : "Google",
          })
        }}
      </p>
      <p>
        {{ t("account.oauth.deadline", { date: formatDate(flow.expiresAt) }) }}
      </p>
      <p>{{ t("account.oauth.returnHint") }}</p>
      <NCollapse
        :key="flow.flowId"
        class="oauth-fallback"
        display-directive="if"
      >
        <NCollapseItem
          name="confirmation"
          :title="t('account.oauth.manualFallback')"
        >
          <AccountOAuthConfirmation
            :flow-id="flow.flowId"
            :disabled="working"
          />
        </NCollapseItem>
      </NCollapse>
      <NButton :disabled="working" @click="cancel">
        {{ t("account.actions.cancelLogin") }}
      </NButton>
    </NAlert>
    <AppDialog
      v-model:open="open"
      :title="
        t(registering ? 'account.actions.register' : 'account.actions.login')
      "
      :locked="working"
      @after-leave="clearPassword"
    >
      <NAlert v-if="error" type="error" role="alert">{{ error }}</NAlert>
      <NForm
        v-if="open"
        ref="form"
        :model="model"
        :rules="rules"
        label-placement="top"
        @submit.prevent="submit"
      >
        <NFormItem :label="t('account.form.email')" path="email">
          <NInput
            v-model:value="model.email"
            :disabled="working"
            autocomplete="username"
            :input-props="{ type: 'email' }"
            :maxlength="254"
          />
        </NFormItem>
        <NFormItem
          ref="passwordItem"
          :label="t('account.form.password')"
          path="password"
        >
          <NInput
            v-model:value="model.password"
            :disabled="working"
            type="password"
            show-password-on="click"
            :autocomplete="registering ? 'new-password' : 'current-password'"
          />
        </NFormItem>
        <NFormItem
          v-if="registering"
          ref="confirmationItem"
          :label="t('account.form.confirmation')"
          path="confirmation"
        >
          <NInput
            v-model:value="model.confirmation"
            :disabled="working"
            type="password"
            show-password-on="click"
            autocomplete="new-password"
          />
        </NFormItem>
        <button type="submit" hidden tabindex="-1" />
      </NForm>
      <template #footer>
        <div class="account-actions account-dialog-actions">
          <NButton :disabled="working" @click="open = false">
            {{ t("common.actions.cancel") }}
          </NButton>
          <NButton
            type="primary"
            :loading="working"
            :disabled="blocked"
            @click="submit"
          >
            {{
              t(
                registering
                  ? "account.actions.register"
                  : "account.actions.login",
              )
            }}
          </NButton>
        </div>
      </template>
    </AppDialog>
  </section>
</template>

<style scoped>
.account-panel {
  max-width: 760px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-bottom: 24px;
}
.account-panel h3,
.account-panel p {
  margin: 0;
}
.account-actions,
.account-summary {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}
.account-summary {
  margin-bottom: 8px;
  overflow-wrap: anywhere;
}
.account-actions {
  margin-top: 12px;
}
.identity-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-top: 10px;
}
.account-dialog-actions {
  justify-content: flex-end;
}
.oauth-fallback {
  margin-top: 12px;
}
</style>

<script setup lang="ts">
import { t } from "../i18n";
import { computed, ref, reactive, watch } from "vue";
import {
  NAlert,
  NButton,
  NCard,
  NForm,
  type FormInst,
  type FormItemInst,
  type FormRules,
  NFormItem,
  NInput,
  NSpin,
  NText,
} from "naive-ui";
import { validateForm } from "../services/forms/rules";
import ApplicationMenu from "./ApplicationMenu.vue";
import WindowControls from "./WindowControls.vue";
import { isMac } from "../services/commands/registry";
import { desktop, errorMessage } from "../services/desktop/client";
import type { VaultStatus } from "../services/desktop/types";
import { useAccount } from "../modules/account/useAccount";
import { useOfficialSync } from "../modules/account/useOfficialSync";
import BrowserAccountLogin from "../modules/account/BrowserAccountLogin.vue";

const props = defineProps<{ initialize: () => Promise<void> }>();
const emit = defineEmits<{ "access-change": [ready: boolean] }>();
const {
  account,
  providers,
  flow,
  loading,
  working,
  busy: accountBusy,
  blocked,
  error: accountError,
  providerError,
  loadProviders,
  perform,
  start,
  cancel,
} = useAccount();
const {
  sync,
  error: syncError,
  refresh: refreshSync,
  invalidate: invalidateSync,
} = useOfficialSync();
const status = ref<VaultStatus>();
const busy = ref(false),
  ready = ref(false),
  initialized = ref(false);
watch(ready, (value) => emit("access-change", value), { immediate: true });
const error = ref(""),
  warning = ref("");
const choice = ref<"local" | "official" | null>(null);
const password = ref(""),
  confirmation = ref("");
const formModel = reactive({ password, confirmation });
const form = ref<FormInst>();
const confirmationItem = ref<FormItemInst>();
const passwordItem = ref<FormItemInst>();
let inspectGeneration = 0;
let inspectedAccountId: string | undefined;
let initializePending: Promise<void> | undefined;
const formBusy = computed(() => busy.value || accountBusy.value);
const checking = computed(() => busy.value || loading.value);
const official = computed(() => !!account.value.user);
const setup = computed(() => status.value?.mode === "setup");
const choosingMode = computed(
  () => !official.value && setup.value && choice.value === null,
);
const choosingLogin = computed(
  () => !official.value && choice.value === "official",
);
const unlockingLocal = computed(
  () => official.value && status.value?.mode === "locked",
);
const needsOfficialPassword = computed(
  () =>
    official.value &&
    !unlockingLocal.value &&
    sync.value?.phase === "needsPassword",
);
const creating = computed(
  () => setup.value && (!official.value || !sync.value?.cloudHasData),
);
const showPassword = computed(
  () =>
    !!status.value &&
    !choosingMode.value &&
    !choosingLogin.value &&
    (unlockingLocal.value ||
      needsOfficialPassword.value ||
      (!official.value && status.value.mode !== "unlocked")),
);
const title = computed(() =>
  choosingMode.value
    ? t("account.mode.title")
    : choosingLogin.value
      ? t("account.mode.official")
      : !showPassword.value
        ? checking.value
          ? t("vault.access.checkingTitle")
          : t("vault.access.failedTitle")
        : unlockingLocal.value
          ? t("account.sync.localUnlockTitle")
          : needsOfficialPassword.value && sync.value?.cloudHasData
            ? t("account.sync.unlockTitle")
            : creating.value
              ? t("vault.setup.title")
              : t("vault.unlock.title"),
);
const explanation = computed(() =>
  !showPassword.value
    ? checking.value
      ? t("vault.access.checkingDescription")
      : t("vault.access.failedDescription")
    : unlockingLocal.value
      ? t("account.sync.localUnlockDescription")
      : official.value
        ? sync.value?.cloudHasData
          ? t("account.sync.unlockDescription")
          : setup.value
            ? t("account.sync.setupDescription")
            : t("account.sync.existingDescription")
        : setup.value
          ? t("vault.setup.warning")
          : t("vault.unlock.automaticProgress"),
);
watch(password, () => {
  if (creating.value && confirmation.value)
    void confirmationItem.value
      ?.validate({ trigger: "input" })
      .catch(() => undefined);
});
const rules = computed<FormRules>(() => ({
  password: {
    required: true,
    trigger: ["input", "blur"],
    validator: () => {
      if (!password.value)
        return new Error(t("vault.validation.passwordRequired"));
      if (
        creating.value &&
        ([...password.value].length < 12 ||
          new TextEncoder().encode(password.value).length > 1024)
      )
        return new Error(t("vault.validation.passwordLength"));
      return true;
    },
  },
  confirmation: {
    required: true,
    trigger: ["input", "blur"],
    validator: () =>
      (!!confirmation.value && confirmation.value === password.value) ||
      new Error(t("vault.validation.passwordMismatch")),
  },
}));
async function enter(current: number) {
  if (!initialized.value) {
    // 多个状态通知共用一次初始化；较旧请求完成后不得越过新的账号门禁。
    initializePending ??= props.initialize();
    try {
      await initializePending;
      initialized.value = true;
    } finally {
      initializePending = undefined;
    }
  }
  if (
    current === inspectGeneration &&
    inspectedAccountId === account.value.user?.id &&
    status.value?.mode === "unlocked" &&
    (!official.value || (!syncError.value && sync.value?.phase === "ready"))
  )
    ready.value = true;
}
async function inspect() {
  const current = ++inspectGeneration;
  busy.value = true;
  error.value = "";
  try {
    // 先恢复本机保险库，再读取加密账号缓存，避免把自动解锁前的空摘要当作离线模式。
    const next = await desktop.vaultStatus();
    if (current !== inspectGeneration) return;
    status.value = next;
    const currentAccount = await desktop.getAccountStatus();
    if (current !== inspectGeneration) return;
    // 本轮读取恢复出的账号已由本轮检查处理，不再让账号监听重复发起检查。
    inspectedAccountId = currentAccount.user?.id;
    account.value = currentAccount;
    accountError.value = "";
    warning.value = [next.keychainWarning, next.cleanupWarning]
      .filter(Boolean)
      .join(" ");
    if (official.value) {
      await refreshSync();
      if (current !== inspectGeneration) return;
      if (
        !syncError.value &&
        sync.value?.phase === "ready" &&
        next.mode === "unlocked"
      )
        await enter(current);
    } else {
      sync.value = undefined;
      syncError.value = "";
      if (next.mode === "unlocked") await enter(current);
    }
  } catch (cause) {
    if (current === inspectGeneration) error.value = errorMessage(cause);
  } finally {
    if (current === inspectGeneration) busy.value = false;
  }
}
watch(
  [() => account.value.user?.id, loading],
  ([userId, isLoading], [, wasLoading]) => {
    if (isLoading || (!wasLoading && userId === inspectedAccountId)) return;
    // 切换账号时保留终端宿主，但先遮住工作台，直至云端密码校验与合并完成。
    ready.value = false;
    password.value = "";
    confirmation.value = "";
    invalidateSync();
    void inspect();
  },
);
watch(
  () => sync.value?.phase,
  (phase) => {
    if (!official.value) return;
    if (phase !== "ready") ready.value = false;
    else if (!checking.value && !ready.value) void inspect();
  },
);
async function chooseLocal() {
  if (formBusy.value) return;
  if (flow.value) await cancel();
  if (official.value) await perform(desktop.logoutAccount);
  if (account.value.user) return;
  choice.value = "local";
  await inspect();
}
async function chooseOfficial() {
  choice.value = "official";
  await loadProviders();
}
async function submit() {
  if (formBusy.value || !(await validateForm(form.value))) return;
  busy.value = true;
  error.value = "";
  try {
    if (unlockingLocal.value) {
      // 本机旧密码可能与云端不同；先解锁原库，云端密码留给下一阶段验证。
      status.value = await desktop.unlockVault(password.value, false);
    } else if (official.value) {
      sync.value = await desktop.completeOfficialSync(password.value);
    } else {
      status.value = await desktop.unlockVault(password.value, setup.value);
    }
    await inspect();
  } catch (cause) {
    error.value = errorMessage(cause);
    try {
      status.value = await desktop.vaultStatus();
    } catch {
      /* 保留原错误。 */
    }
  } finally {
    // 密码仅在输入与提交期间保留；JavaScript 字符串无法保证原位擦除。
    password.value = "";
    confirmation.value = "";
    busy.value = false;
    if (!ready.value) {
      void passwordItem.value
        ?.validate({ trigger: "input" })
        .catch(() => undefined);
      if (creating.value)
        void confirmationItem.value
          ?.validate({ trigger: "input" })
          .catch(() => undefined);
    }
  }
}
</script>
<template>
  <div
    v-if="initialized"
    v-show="ready"
    :inert="!ready || undefined"
    class="vault-workbench"
  >
    <slot />
  </div>
  <NAlert
    v-if="ready && warning"
    class="vault-warning"
    type="warning"
    closable
    @close="warning = ''"
  >
    {{ warning }}
  </NAlert>
  <div v-if="!ready" class="vault-gate" :class="{ 'platform-mac': isMac }">
    <header class="vault-titlebar" data-tauri-drag-region>
      <ApplicationMenu v-if="!isMac" />
      <span data-tauri-drag-region>Termeta</span>
      <div class="vault-drag-space" data-tauri-drag-region />
      <WindowControls @error="error = $event" />
    </header>
    <main class="vault-content">
      <NCard :title="title" class="vault-card">
        <NSpin :show="busy || loading">
          <template v-if="choosingMode">
            <NText tag="p" depth="3">{{ t("account.mode.description") }}</NText>
            <div class="mode-actions">
              <NButton
                type="primary"
                :disabled="formBusy"
                @click="chooseOfficial"
              >
                {{ t("account.mode.official") }}
              </NButton>
              <NButton :disabled="formBusy" @click="chooseLocal">
                {{ t("account.mode.local") }}
              </NButton>
            </div>
          </template>
          <BrowserAccountLogin
            v-else-if="choosingLogin"
            :providers="providers"
            :flow="flow"
            :blocked="blocked"
            :working="working"
            :provider-error="providerError"
            @start="start"
            @cancel="cancel"
            @retry="loadProviders"
          />
          <template v-else>
            <p v-if="official" class="account-email">
              {{ account.user?.email }}
            </p>
            <NText tag="p" depth="3">{{ explanation }}</NText>
            <NAlert v-if="warning" type="warning">{{ warning }}</NAlert>
            <NForm
              v-if="showPassword"
              ref="form"
              :model="formModel"
              :rules="rules"
              @submit.prevent="submit"
            >
              <NFormItem
                ref="passwordItem"
                path="password"
                :label="
                  creating
                    ? t('vault.setup.passwordPlaceholder')
                    : t('vault.form.password')
                "
              >
                <NInput
                  v-model:value="formModel.password"
                  type="password"
                  show-password-on="click"
                  :disabled="formBusy"
                  :input-props="{
                    autocomplete: creating
                      ? 'new-password'
                      : 'current-password',
                    maxlength: 1024,
                  }"
                  autofocus
                />
              </NFormItem>
              <NFormItem
                v-if="creating"
                ref="confirmationItem"
                :label="t('vault.form.confirmPassword')"
                path="confirmation"
              >
                <NInput
                  v-model:value="formModel.confirmation"
                  type="password"
                  :disabled="formBusy"
                  :input-props="{
                    autocomplete: 'new-password',
                    maxlength: 1024,
                  }"
                />
              </NFormItem>
              <NButton
                type="primary"
                attr-type="submit"
                :loading="busy"
                :disabled="formBusy"
              >
                {{
                  official && !unlockingLocal
                    ? t("account.sync.continue")
                    : creating
                      ? t("vault.actions.createAndUnlock")
                      : t("vault.actions.unlock")
                }}
              </NButton>
            </NForm>
          </template>
          <NAlert
            v-if="error || accountError || syncError || sync?.error"
            type="error"
            class="vault-error"
          >
            {{ error || accountError || syncError || sync?.error }}
          </NAlert>
          <div v-if="choosingLogin || official" class="mode-actions">
            <NButton :disabled="formBusy" @click="chooseLocal">
              {{
                official
                  ? t("account.mode.logoutLocal")
                  : t("account.mode.local")
              }}
            </NButton>
          </div>
          <NButton
            v-if="!formBusy && !showPassword && !choosingMode && !choosingLogin"
            :disabled="formBusy"
            @click="inspect"
          >
            {{ t("common.actions.retry") }}
          </NButton>
        </NSpin>
      </NCard>
    </main>
  </div>
</template>
<style scoped>
.vault-gate {
  height: 100dvh;
  display: flex;
  flex-direction: column;
  background: var(--bg);
}
.vault-titlebar {
  height: 48px;
  padding: 0 12px;
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  color: var(--muted);
}
.platform-mac .vault-titlebar {
  padding-left: 88px;
}
.vault-drag-space {
  flex: 1;
  align-self: stretch;
}
.vault-content {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: grid;
  place-items: center;
  padding: 28px;
}
.vault-card {
  width: min(520px, 100%);
}
.vault-workbench {
  display: contents;
}
.mode-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 20px;
}
.account-email {
  overflow-wrap: anywhere;
}
.vault-error {
  margin-top: 16px;
}
.vault-warning {
  position: fixed;
  z-index: 4000;
  top: 62px;
  left: 50%;
  transform: translateX(-50%);
  width: min(640px, calc(100% - 32px));
}
</style>

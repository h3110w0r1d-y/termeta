<script setup lang="ts">
import { t } from "../i18n";
import {
  computed,
  onBeforeUnmount,
  onMounted,
  ref,
  reactive,
  watch,
} from "vue";
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
const props = defineProps<{ initialize: () => Promise<void> }>();
const emit = defineEmits<{ "access-change": [ready: boolean] }>();
const status = ref<VaultStatus>();
const busy = ref(false),
  ready = ref(false);
watch(ready, (value) => emit("access-change", value), { immediate: true });
const error = ref(""),
  warning = ref("");
const password = ref(""),
  confirmation = ref("");
const formModel = reactive({ password, confirmation });
const form = ref<FormInst>();
const confirmationItem = ref<FormItemInst>();
const passwordItem = ref<FormItemInst>();
let disposed = false;
onBeforeUnmount(() => {
  disposed = true;
});
const setup = computed(() => status.value?.mode === "setup");
const showPassword = computed(
  () => !!status.value && status.value.mode !== "unlocked",
);
const title = computed(() =>
  showPassword.value
    ? setup.value
      ? t("vault.setup.title")
      : t("vault.unlock.title")
    : busy.value
      ? t("vault.access.checkingTitle")
      : t("vault.access.failedTitle"),
);
const explanation = computed(() =>
  showPassword.value
    ? setup.value
      ? t("vault.setup.warning")
      : t("vault.unlock.automaticProgress")
    : busy.value
      ? t("vault.access.checkingDescription")
      : t("vault.access.failedDescription"),
);
watch(password, () => {
  if (setup.value && confirmation.value)
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
        setup.value &&
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
async function inspect() {
  if (disposed) return;
  busy.value = true;
  error.value = "";
  try {
    const next = await desktop.vaultStatus();
    if (disposed) return;
    status.value = next;
    warning.value = [next.keychainWarning, next.cleanupWarning]
      .filter(Boolean)
      .join(" ");
    if (next.mode === "unlocked") {
      await props.initialize();
      if (!disposed) ready.value = true;
    }
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}
onMounted(inspect);

async function submit() {
  if (busy.value || !(await validateForm(form.value))) return;
  busy.value = true;
  error.value = "";
  try {
    status.value = await desktop.unlockVault(password.value, setup.value);
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
      if (setup.value)
        void confirmationItem.value
          ?.validate({ trigger: "input" })
          .catch(() => undefined);
    }
  }
}
</script>
<template>
  <div v-if="ready" class="vault-workbench">
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
        <NSpin :show="busy">
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
                setup
                  ? t('vault.setup.passwordPlaceholder')
                  : t('vault.form.password')
              "
            >
              <NInput
                v-model:value="formModel.password"
                type="password"
                show-password-on="click"
                :disabled="busy"
                :input-props="{
                  autocomplete: setup ? 'new-password' : 'current-password',
                  maxlength: 1024,
                }"
                autofocus
              />
            </NFormItem>
            <NFormItem
              v-if="setup"
              ref="confirmationItem"
              :label="t('vault.form.confirmPassword')"
              path="confirmation"
            >
              <NInput
                v-model:value="formModel.confirmation"
                type="password"
                :disabled="busy"
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
              :disabled="busy"
            >
              {{
                setup
                  ? t("vault.actions.createAndUnlock")
                  : t("vault.actions.unlock")
              }}
            </NButton>
          </NForm>
          <NAlert v-if="error" type="error" class="vault-error">
            {{ error }}
          </NAlert>
          <NButton
            v-if="!busy && !showPassword"
            :disabled="busy"
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

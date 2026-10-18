<script setup lang="ts">
import { t } from "../i18n";
import { computed, onMounted, ref, reactive, watch } from "vue";
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
  NRadioButton,
  NRadioGroup,
  NSpin,
  NText,
} from "naive-ui";
import { validateForm } from "../services/forms/rules";
import ApplicationMenu from "./ApplicationMenu.vue";
import WindowControls from "./WindowControls.vue";
import { isMac } from "../services/commands/registry";
import { desktop, errorMessage } from "../services/desktop/client";
import type { VaultStatus } from "../services/desktop/types";
import BackupPanel from "../modules/backup/BackupPanel.vue";
const props = defineProps<{ initialize: () => Promise<void> }>();
const status = ref<VaultStatus>();
const busy = ref(false),
  ready = ref(false),
  error = ref(""),
  warning = ref("");
const password = ref(""),
  confirmation = ref("");
const formModel = reactive({ password, confirmation });
const setupSource = ref<"local" | "cloud">("local");
const backupBusy = ref(false);
const formBusy = computed(() => busy.value || backupBusy.value);
const setup = computed(() => status.value?.mode === "setup");
const form = ref<FormInst>();
const confirmationItem = ref<FormItemInst>();
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
async function accept(next: VaultStatus) {
  status.value = next;
  warning.value = [next.keychainWarning, next.cleanupWarning]
    .filter(Boolean)
    .join(" ");
  if (next.mode === "unlocked") {
    await props.initialize();
    ready.value = true;
  }
}
async function inspect() {
  busy.value = true;
  error.value = "";
  try {
    await accept(await desktop.vaultStatus());
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
async function submit() {
  if (formBusy.value) return;
  if (setup.value && setupSource.value === "cloud") return;
  error.value = "";
  if (!(await validateForm(form.value))) return;
  busy.value = true;
  try {
    await accept(await desktop.unlockVault(password.value, setup.value));
  } catch (e) {
    error.value = errorMessage(e);
    // 初始化可能已写入派生参数；重新读取状态，允许密码恢复中断的初始化。
    try {
      status.value = await desktop.vaultStatus();
    } catch {
      /* 原错误保留供重试。 */
    }
  } finally {
    // 前端仅在提交期间持有输入；JavaScript 字符串无法保证原位擦除。
    password.value = "";
    confirmation.value = "";
    busy.value = false;
  }
}
async function initializeFromCloud(token: string) {
  if (busy.value || !setup.value) return false;
  if (!(await validateForm(form.value))) return false;
  busy.value = true;
  error.value = "";
  try {
    await accept(
      await desktop.initializeFromCloudBackup(token, password.value),
    );
    return true;
  } catch (cause) {
    error.value = errorMessage(cause);
    // 导入中断时本机保险库可能已创建，读取状态后提供解锁或重新进入入口。
    try {
      status.value = await desktop.vaultStatus();
    } catch {
      /* 保留原错误。 */
    }
    throw cause;
  } finally {
    password.value = "";
    confirmation.value = "";
    busy.value = false;
  }
}
onMounted(inspect);
</script>
<template>
  <template v-if="ready">
    <NAlert
      v-if="warning"
      class="vault-warning"
      type="warning"
      closable
      @close="warning = ''"
    >
      {{ warning }}
    </NAlert>
    <slot />
  </template>
  <div v-else class="vault-gate" :class="{ 'platform-mac': isMac }">
    <header class="vault-titlebar" data-tauri-drag-region>
      <ApplicationMenu v-if="!isMac" />
      <span data-tauri-drag-region>Termeta</span>
      <div class="vault-drag-space" data-tauri-drag-region />
      <WindowControls @error="error = $event" />
    </header>
    <main class="vault-content">
      <NCard
        :title="setup ? t('vault.setup.title') : t('vault.unlock.title')"
        class="vault-card"
        :class="{ 'vault-card-cloud': setup && setupSource === 'cloud' }"
      >
        <NSpin :show="busy">
          <NText tag="p" depth="3">
            {{
              setup
                ? t("vault.setup.warning")
                : t("vault.unlock.automaticProgress")
            }}
          </NText>
          <NAlert v-if="warning" type="warning">{{ warning }}</NAlert>
          <NRadioGroup
            v-if="setup"
            v-model:value="setupSource"
            :disabled="formBusy"
            :aria-label="t('vault.setup.method')"
            class="setup-source"
          >
            <NRadioButton value="local">
              {{ t("vault.setup.empty") }}
            </NRadioButton>
            <NRadioButton value="cloud">
              {{ t("vault.setup.cloud") }}
            </NRadioButton>
          </NRadioGroup>
          <NForm
            ref="form"
            :model="formModel"
            :rules="rules"
            v-if="status && status.mode !== 'unlocked'"
            @submit.prevent="submit"
          >
            <NFormItem
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
                :disabled="formBusy"
                :input-props="{
                  autocomplete: setup ? 'new-password' : 'current-password',
                  maxlength: 1024,
                }"
                autofocus
              />
            </NFormItem>
            <NFormItem
              ref="confirmationItem"
              v-if="setup"
              :label="t('vault.form.confirmPassword')"
              path="confirmation"
            >
              <NInput
                v-model:value="formModel.confirmation"
                type="password"
                :disabled="formBusy"
                :input-props="{ autocomplete: 'new-password', maxlength: 1024 }"
              />
            </NFormItem>
            <NButton
              v-if="!setup || setupSource === 'local'"
              type="primary"
              attr-type="submit"
              :loading="busy"
              :disabled="formBusy"
            >
              {{
                setup
                  ? t("vault.actions.createAndUnlock")
                  : t("vault.actions.unlock")
              }}
            </NButton>
          </NForm>
          <BackupPanel
            v-if="setup && setupSource === 'cloud'"
            initializing
            :disabled="busy"
            :initialize="initializeFromCloud"
            @busy-change="backupBusy = $event"
          />
          <NAlert v-if="error" type="error" class="vault-error">
            {{ error }}
          </NAlert>
          <NButton
            v-if="!status || status.mode === 'unlocked'"
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
.vault-card-cloud {
  width: min(760px, 100%);
}
.setup-source {
  margin-bottom: 20px;
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

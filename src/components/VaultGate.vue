<script setup lang="ts">
import { computed, onMounted, ref, reactive } from "vue";
import {
  NAlert,
  NButton,
  NCard,
  NForm,
  type FormInst,
  type FormRules,
  NFormItem,
  NInput,
  NSpin,
  NText,
} from "naive-ui";
import { validateForm } from "../services/forms/rules";
import WindowControls from "./WindowControls.vue";
import { isMac } from "../services/commands/registry";
import { desktop, errorMessage } from "../services/desktop/client";
import type { VaultStatus } from "../services/desktop/types";
const props = defineProps<{ initialize: () => Promise<void> }>();
const status = ref<VaultStatus>();
const busy = ref(false),
  ready = ref(false),
  error = ref(""),
  warning = ref("");
const password = ref(""),
  confirmation = ref("");
const formModel = reactive({ password, confirmation });
const setup = computed(() => status.value?.mode === "setup");
const form = ref<FormInst>();
const rules = computed<FormRules>(() => ({
  password: {
    required: true,
    trigger: ["input", "blur"],
    validator: () => {
      if (!password.value) return new Error("请输入保险库密码");
      if (
        setup.value &&
        ([...password.value].length < 12 ||
          new TextEncoder().encode(password.value).length > 1024)
      )
        return new Error("密码至少12个字符，UTF-8编码不超过1024字节");
      return true;
    },
  },
  confirmation: {
    required: true,
    trigger: ["input", "blur"],
    validator: () =>
      (!!confirmation.value && confirmation.value === password.value) ||
      new Error("两次输入的密码须一致"),
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
  if (busy.value) return;
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
      <span data-tauri-drag-region>Termeta</span>
      <div class="vault-drag-space" data-tauri-drag-region />
      <WindowControls @error="error = $event" />
    </header>
    <main class="vault-content">
      <NCard
        :title="setup ? '设置数据保险库密码' : '解锁数据保险库'"
        class="vault-card"
      >
        <NSpin :show="busy">
          <NText tag="p" depth="3">
            {{
              setup
                ? "主机配置和凭据将加密保存在本机。请牢记此密码，系统钥匙串不可用时需要用它恢复访问；遗忘密码且丢失钥匙串主密钥将无法恢复数据。"
                : "正在尝试使用系统钥匙串自动解锁。若无法自动解锁，请输入当前保险库密码。"
            }}
          </NText>
          <NAlert v-if="warning" type="warning">{{ warning }}</NAlert>
          <NForm
            ref="form"
            :model="formModel"
            :rules="rules"
            v-if="status && status.mode !== 'unlocked'"
            @submit.prevent="submit"
          >
            <NFormItem
              path="password"
              :label="setup ? '设置密码（至少12个字符）' : '保险库密码'"
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
            <NFormItem v-if="setup" label="确认密码" path="confirmation">
              <NInput
                v-model:value="formModel.confirmation"
                type="password"
                :disabled="busy"
                :input-props="{ autocomplete: 'new-password', maxlength: 1024 }"
              />
            </NFormItem>
            <NButton
              type="primary"
              attr-type="submit"
              :loading="busy"
              :disabled="busy"
            >
              {{ setup ? "创建并解锁" : "解锁" }}
            </NButton>
          </NForm>
          <NAlert v-if="error" type="error" class="vault-error">
            {{ error }}
          </NAlert>
          <NButton
            v-if="!status || status.mode === 'unlocked'"
            :disabled="busy"
            @click="inspect"
          >
            重试
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
  height: 54px;
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

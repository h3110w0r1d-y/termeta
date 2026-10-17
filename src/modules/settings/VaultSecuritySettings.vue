<script setup lang="ts">
import { onBeforeUnmount, reactive, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NForm,
  NFormItem,
  NInput,
  type FormInst,
  type FormItemInst,
  type FormRules,
} from "naive-ui";
import AppDialog from "../../components/ui/AppDialog.vue";
import { validateForm } from "../../services/forms/rules";
import { desktop, errorMessage } from "../../services/desktop/client";

const open = ref(false);
const busy = ref(false);
const error = ref("");
const success = ref(false);
const warnings = ref<string[]>([]);
const form = ref<FormInst>();
const confirmationItem = ref<FormItemInst>();
const model = reactive({ oldPassword: "", newPassword: "", confirmation: "" });
defineExpose({ busy });

const rules: FormRules = {
  oldPassword: {
    required: true,
    trigger: ["input", "blur"],
    validator: (_, value: string) =>
      (value.length > 0 && new TextEncoder().encode(value).length <= 1024) ||
      new Error("请输入旧密码，UTF-8编码不超过1024字节"),
  },
  newPassword: {
    required: true,
    trigger: ["input", "blur"],
    validator: (_, value: string) => {
      if (
        [...value].length < 12 ||
        new TextEncoder().encode(value).length > 1024
      )
        return new Error("新密码至少12个字符，UTF-8编码不超过1024字节");
      return value !== model.oldPassword || new Error("新密码不能与旧密码相同");
    },
  },
  confirmation: {
    required: true,
    trigger: ["input", "blur"],
    validator: (_, value: string) =>
      (value.length > 0 && value === model.newPassword) ||
      new Error("两次输入的新密码须一致"),
  },
};
watch(
  () => model.newPassword,
  () => {
    if (model.confirmation)
      void confirmationItem.value
        ?.validate({ trigger: "input" })
        .catch(() => undefined);
  },
);
function clear() {
  model.oldPassword = "";
  model.newPassword = "";
  model.confirmation = "";
}
function begin() {
  clear();
  error.value = "";
  open.value = true;
}
async function submit() {
  if (busy.value) return;
  busy.value = true;
  if (!(await validateForm(form.value))) {
    busy.value = false;
    return;
  }
  error.value = "";
  success.value = false;
  warnings.value = [];
  try {
    const result = await desktop.changeVaultPassword(
      model.oldPassword,
      model.newPassword,
    );
    success.value = true;
    warnings.value = [result.keychainWarning, result.cleanupWarning].filter(
      (warning): warning is string => warning !== null,
    );
    open.value = false;
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    // 仅在表单和提交期间持有密码；JavaScript 字符串无法保证原位擦除。
    clear();
    busy.value = false;
  }
}
onBeforeUnmount(clear);
</script>

<template>
  <section class="vault-security">
    <h3>保险库密码</h3>
    <p class="muted">修改密码将重新加密本机保存的主机、凭据、设置和历史。</p>
    <NAlert v-if="success" type="success">保险库密码已修改。</NAlert>
    <NAlert v-for="warning in warnings" :key="warning" type="warning">
      {{ warning }}
    </NAlert>
    <NButton :disabled="busy" @click="begin">修改保险库密码</NButton>
    <AppDialog
      v-model:open="open"
      title="修改保险库密码"
      description="请牢记新密码。已有数据库文件副本仍使用创建时的保险库密码；云端备份使用独立备份密码，不随本次修改变化。"
      :locked="busy"
      @after-leave="clear"
    >
      <NForm
        ref="form"
        :model="model"
        :rules="rules"
        label-placement="top"
        @submit.prevent="submit"
      >
        <NFormItem label="旧密码" path="oldPassword">
          <NInput
            v-model:value="model.oldPassword"
            type="password"
            show-password-on="click"
            :disabled="busy"
            :input-props="{ autocomplete: 'current-password', maxlength: 1024 }"
            autofocus
          />
        </NFormItem>
        <NFormItem label="新密码" path="newPassword">
          <NInput
            v-model:value="model.newPassword"
            type="password"
            show-password-on="click"
            :disabled="busy"
            :input-props="{ autocomplete: 'new-password', maxlength: 1024 }"
          />
        </NFormItem>
        <NFormItem
          ref="confirmationItem"
          label="确认新密码"
          path="confirmation"
        >
          <NInput
            v-model:value="model.confirmation"
            type="password"
            :disabled="busy"
            :input-props="{ autocomplete: 'new-password', maxlength: 1024 }"
          />
        </NFormItem>
      </NForm>
      <NAlert v-if="busy" type="info">正在重新加密，请稍候…</NAlert>
      <NAlert v-if="error" type="error">{{ error }}</NAlert>
      <template #footer>
        <NButton :disabled="busy" @click="open = false">取消</NButton>
        <NButton
          type="primary"
          :loading="busy"
          :disabled="busy"
          @click="submit"
        >
          确认修改
        </NButton>
      </template>
    </AppDialog>
  </section>
</template>

<style scoped>
.vault-security {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 12px;
}
.vault-security h3,
.vault-security p {
  margin: 0;
}
.vault-security :deep(.n-alert) {
  align-self: stretch;
}
</style>

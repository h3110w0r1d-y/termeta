<script setup lang="ts">
import { onBeforeUnmount, reactive, ref } from "vue";
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
import { t } from "@/i18n";
import { desktop, errorMessage } from "@/services/desktop/client";
import { validateForm } from "@/services/forms/rules";

const props = defineProps<{ flowId: string; disabled: boolean }>();
const model = reactive({ confirmationCode: "" });
const form = ref<FormInst>();
const codeItem = ref<FormItemInst>();
const busy = ref(false);
const submitted = ref(false);
const error = ref("");
let disposed = false;
const rules: FormRules = {
  confirmationCode: {
    required: true,
    trigger: ["input", "change", "blur"],
    validator: (_, value: string) =>
      /^[A-Za-z0-9_-]{32}$/.test(value.trim()) ||
      new Error(t("account.oauth.validationCode")),
  },
};
async function submit() {
  if (disposed || busy.value || props.disabled || submitted.value) return;
  busy.value = true;
  if (!(await validateForm(form.value))) {
    busy.value = false;
    return;
  }
  if (disposed) return;
  error.value = "";
  try {
    const request = desktop.confirmAccountOAuth(
      props.flowId,
      model.confirmationCode.trim(),
    );
    // 确认码仅在输入时短暂存在，交给 Rust 后立即清除，不等待 IPC 返回。
    model.confirmationCode = "";
    await request;
    if (!disposed) submitted.value = true;
  } catch (cause) {
    if (!disposed) error.value = errorMessage(cause);
  } finally {
    model.confirmationCode = "";
    if (!disposed) {
      busy.value = false;
      if (!submitted.value)
        void codeItem.value
          ?.validate({ trigger: "change" })
          .catch(() => undefined);
    }
  }
}
onBeforeUnmount(() => {
  disposed = true;
  model.confirmationCode = "";
});
</script>

<template>
  <div class="oauth-confirmation">
    <p>{{ t("account.oauth.codeDescription") }}</p>
    <p>{{ t("account.oauth.codeSecurity") }}</p>
    <NAlert v-if="error" type="error" role="alert">{{ error }}</NAlert>
    <p v-if="submitted" role="status">{{ t("account.oauth.codeSubmitted") }}</p>
    <NForm
      v-else
      ref="form"
      :model="model"
      :rules="rules"
      label-placement="top"
      @submit.prevent="submit"
    >
      <NFormItem
        ref="codeItem"
        :label="t('account.oauth.confirmationCode')"
        path="confirmationCode"
      >
        <NInput
          v-model:value="model.confirmationCode"
          type="password"
          show-password-on="click"
          autocomplete="off"
          :input-props="{ autocapitalize: 'none', spellcheck: false }"
          :disabled="disabled || busy"
          :maxlength="64"
        />
      </NFormItem>
      <NButton attr-type="submit" :loading="busy" :disabled="disabled || busy">
        {{ t("account.oauth.submitCode") }}
      </NButton>
    </NForm>
  </div>
</template>

<style scoped>
.oauth-confirmation {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 14px 0;
}
.oauth-confirmation p {
  margin: 0;
}
</style>

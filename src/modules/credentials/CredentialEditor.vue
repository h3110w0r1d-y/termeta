<script setup lang="ts">
import { t } from "@/i18n";
import {
  NFlex,
  NForm,
  NFormItem,
  type FormInst,
  type FormRules,
} from "naive-ui";
import { computed, reactive, ref, useId, watch } from "vue";
import AppButton from "@/components/ui/AppButton.vue";
import AppDialog from "@/components/ui/AppDialog.vue";
import AppInput from "@/components/ui/AppInput.vue";
import AppSelect from "@/components/ui/AppSelect.vue";
import AppTextarea from "@/components/ui/AppTextarea.vue";
import { requiredText, validateForm } from "@/services/forms/rules";
import { desktop, errorMessage } from "@/services/desktop/client";

const props = withDefaults(
  defineProps<{
    open: boolean;
    initialKind?: "password" | "privateKey";
  }>(),
  { initialKind: "password" },
);
const emit = defineEmits<{
  "update:open": [boolean];
  saved: [string];
}>();

const form = ref<FormInst>();
const formId = useId();
const busy = ref(false);
const error = ref("");
const label = ref("");
const secret = ref("");
const passphrase = ref("");
const privateKey = ref("");
const kind = ref<"password" | "privateKey">("password");
const formModel = reactive({ label, kind, secret, privateKey, passphrase });

watch(kind, clearSecrets, { flush: "sync" });
watch(
  () => props.open,
  (open) => {
    if (!open) {
      clearSecrets();
      return;
    }
    label.value = "";
    kind.value = props.initialKind;
    clearSecrets();
    error.value = "";
  },
);

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
  privateKey:
    kind.value === "privateKey"
      ? requiredText("credentials.form.sshPrivateKey")
      : [],
}));

function clearSecrets() {
  secret.value = "";
  passphrase.value = "";
  privateKey.value = "";
}

function close(open: boolean) {
  if (busy.value && !open) return;
  emit("update:open", open);
  if (!open) clearSecrets();
}

async function save() {
  if (busy.value || !(await validateForm(form.value))) return;
  busy.value = true;
  error.value = "";
  try {
    const result = await desktop.saveCredential({
      kind: kind.value,
      label: label.value.trim(),
      ...(kind.value === "password"
        ? { password: secret.value }
        : {
            privateKey: privateKey.value,
            passphrase: passphrase.value || undefined,
          }),
    });
    emit("saved", result.id);
    emit("update:open", false);
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    clearSecrets();
    busy.value = false;
  }
}
</script>

<template>
  <AppDialog
    :open="open"
    :title="t('credentials.actions.new')"
    :description="t('credentials.form.description')"
    :locked="busy"
    @update:open="close"
  >
    <NForm
      ref="form"
      :model="formModel"
      :rules="rules"
      label-placement="top"
      novalidate
      :id="formId"
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
          class="span-2"
          :label="t('credentials.form.sshPrivateKey')"
          path="privateKey"
        >
          <AppTextarea
            v-model="formModel.privateKey"
            :disabled="busy"
            :rows="8"
            :maxlength="1048576"
            autocomplete="off"
            autocapitalize="off"
            :spellcheck="false"
            :placeholder="t('credentials.form.privateKeyPlaceholder')"
          />
        </NFormItem>
        <p class="muted span-2">{{ t("credentials.form.keyHint") }}</p>
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
        <AppButton type="button" :disabled="busy" @click="close(false)">
          {{ t("common.actions.cancel") }}
        </AppButton>
        <AppButton
          type="submit"
          :form="formId"
          class="primary"
          :disabled="busy"
        >
          {{ busy ? t("common.status.saving") : t("credentials.actions.save") }}
        </AppButton>
      </NFlex>
    </template>
  </AppDialog>
</template>

<script setup lang="ts">
import { onBeforeUnmount, reactive, ref, useId, watch } from "vue";
import { NFlex, NForm, NFormItem, NSpin } from "naive-ui";
import { t } from "@/i18n";
import AppButton from "@/components/ui/AppButton.vue";
import AppDialog from "@/components/ui/AppDialog.vue";
import AppInput from "@/components/ui/AppInput.vue";
import AppTextarea from "@/components/ui/AppTextarea.vue";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { CredentialMetadata } from "@/services/desktop/types";

const props = defineProps<{ credential: CredentialMetadata | null }>();
const emit = defineEmits<{ close: [] }>();
const publicKey = ref("");
const error = ref("");
const loading = ref(false);
const copied = ref(false);
const copying = ref(false);
const passphrase = ref("");
const formModel = reactive({ passphrase });
const formId = useId();
let generation = 0;

watch(
  () => props.credential?.id,
  (id) => {
    generation += 1;
    publicKey.value = "";
    error.value = "";
    copied.value = false;
    copying.value = false;
    passphrase.value = "";
    loading.value = false;
    if (id) void readPublicKey();
  },
  { immediate: true, flush: "sync" },
);
onBeforeUnmount(() => {
  generation += 1;
  passphrase.value = "";
});

async function readPublicKey() {
  const id = props.credential?.id;
  if (!id || loading.value) return;
  const requestGeneration = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const key = await desktop.credentialPublicKey(
      id,
      passphrase.value || undefined,
    );
    // 关闭、切换或重试后丢弃迟到结果，避免覆盖当前凭据的公钥。
    if (generation === requestGeneration) publicKey.value = key;
  } catch (cause) {
    if (generation === requestGeneration) error.value = errorMessage(cause);
  } finally {
    if (generation === requestGeneration) {
      passphrase.value = "";
      loading.value = false;
    }
  }
}

async function copy() {
  if (!publicKey.value || copying.value) return;
  const requestGeneration = generation;
  copying.value = true;
  copied.value = false;
  error.value = "";
  try {
    await navigator.clipboard.writeText(publicKey.value);
    if (generation === requestGeneration) copied.value = true;
  } catch (cause) {
    if (generation === requestGeneration) error.value = errorMessage(cause);
  } finally {
    if (generation === requestGeneration) copying.value = false;
  }
}
</script>

<template>
  <AppDialog
    :open="credential !== null"
    :title="t('credentials.publicKey.title')"
    :description="credential?.label"
    @update:open="(open) => !open && emit('close')"
  >
    <NSpin
      v-if="loading"
      size="small"
      :description="t('credentials.publicKey.loading')"
    />
    <template v-else-if="publicKey">
      <AppTextarea
        :model-value="publicKey"
        readonly
        :rows="5"
        :spellcheck="false"
        :aria-label="t('credentials.publicKey.title')"
      />
      <p class="muted">{{ t("credentials.publicKey.hint") }}</p>
    </template>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <NForm
      v-if="!loading && !publicKey"
      :id="formId"
      :model="formModel"
      label-placement="top"
      @submit.prevent="readPublicKey"
    >
      <NFormItem :label="t('credentials.form.passphrase')" path="passphrase">
        <AppInput
          v-model="formModel.passphrase"
          type="password"
          autocomplete="new-password"
          :maxlength="65536"
        />
      </NFormItem>
      <p class="muted">{{ t("credentials.publicKey.passphraseHint") }}</p>
    </NForm>
    <p v-if="copied" role="status">{{ t("credentials.publicKey.copied") }}</p>
    <template #footer>
      <NFlex justify="end">
        <AppButton @click="emit('close')">
          {{ t("common.actions.close") }}
        </AppButton>
        <AppButton
          v-if="!loading && !publicKey"
          type="submit"
          :form="formId"
          class="primary"
        >
          {{ t("common.actions.retry") }}
        </AppButton>
        <AppButton
          v-else
          class="primary"
          :disabled="loading || !publicKey || copying"
          @click="copy"
        >
          {{ t("credentials.actions.copyPublicKey") }}
        </AppButton>
      </NFlex>
    </template>
  </AppDialog>
</template>

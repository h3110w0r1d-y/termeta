<script setup lang="ts">
import { ref, watch } from "vue";
import { NFlex, NSpin } from "naive-ui";
import { t } from "@/i18n";
import AppButton from "@/components/ui/AppButton.vue";
import AppDialog from "@/components/ui/AppDialog.vue";
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
let generation = 0;

watch(
  () => props.credential?.id,
  async (id, _, onCleanup) => {
    generation += 1;
    let current = true;
    onCleanup(() => {
      current = false;
    });
    publicKey.value = "";
    error.value = "";
    copied.value = false;
    copying.value = false;
    loading.value = !!id;
    if (!id) return;
    try {
      const key = await desktop.credentialPublicKey(id);
      // 关闭或切换凭据后丢弃迟到结果，避免显示上一项的公钥。
      if (current) publicKey.value = key;
    } catch (cause) {
      if (current) error.value = errorMessage(cause);
    } finally {
      if (current) loading.value = false;
    }
  },
  { immediate: true },
);

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
    <p v-if="copied" role="status">{{ t("credentials.publicKey.copied") }}</p>
    <template #footer>
      <NFlex justify="end">
        <AppButton @click="emit('close')">
          {{ t("common.actions.close") }}
        </AppButton>
        <AppButton
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

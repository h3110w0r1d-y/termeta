<script setup lang="ts">
import { t } from "../../i18n";
import { computed, ref, watch } from "vue";
import { validateForm } from "../../services/forms/rules";
import {
  NFlex,
  NForm,
  NFormItem,
  type FormInst,
  type FormRules,
} from "naive-ui";
import AppButton from "./AppButton.vue";
import AppInput from "./AppInput.vue";
import AppDialog from "./AppDialog.vue";
import { currentConfirmation, settleConfirmation } from "./confirmation";
const form = ref<FormInst>();
const rules = computed<FormRules>(() => ({
  input: { max: 120, message: t("最多120个字符"), trigger: ["input", "blur"] },
}));
watch(currentConfirmation, () => form.value?.restoreValidation());
async function submit() {
  if (await validateForm(form.value)) settleConfirmation(true);
}
</script>
<template>
  <AppDialog
    :open="!!currentConfirmation"
    :title="
      currentConfirmation?.input === undefined ? t('确认操作') : t('输入信息')
    "
    @update:open="(value) => !value && settleConfirmation(false)"
  >
    <NForm
      ref="form"
      :model="currentConfirmation"
      :rules="rules"
      id="confirmation-form"
      v-if="currentConfirmation"
      @submit.prevent="submit"
    >
      <p class="confirmation-message">{{ currentConfirmation.message }}</p>
      <NFormItem
        v-if="currentConfirmation.input !== undefined"
        :label="t('输入信息')"
        path="input"
      >
        <AppInput
          v-model="currentConfirmation.input"
          maxlength="120"
          autofocus
        />
      </NFormItem>
    </NForm>
    <template #footer>
      <NFlex justify="end">
        <AppButton type="button" @click="settleConfirmation(false)">
          {{ t("取消") }}
        </AppButton>
        <AppButton type="submit" form="confirmation-form" class="primary">
          {{ t("确认") }}
        </AppButton>
      </NFlex>
    </template>
  </AppDialog>
</template>

<style scoped>
.confirmation-message {
  white-space: pre-wrap;
  line-height: 1.7;
}
</style>

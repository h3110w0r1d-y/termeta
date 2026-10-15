<script setup lang="ts">
import { NFlex } from "naive-ui";
import AppButton from "./AppButton.vue";
import AppInput from "./AppInput.vue";
import AppDialog from "./AppDialog.vue";
import { currentConfirmation, settleConfirmation } from "./confirmation";
</script>
<template>
  <AppDialog
    :open="!!currentConfirmation"
    :title="currentConfirmation?.input === undefined ? '确认操作' : '输入信息'"
    @update:open="(value) => !value && settleConfirmation(false)"
    ><form
      id="confirmation-form"
      v-if="currentConfirmation"
      @submit.prevent="settleConfirmation(true)"
    >
      <p class="confirmation-message">{{ currentConfirmation.message }}</p>
      <AppInput
        v-if="currentConfirmation.input !== undefined"
        v-model="currentConfirmation.input"
        maxlength="120"
        autofocus
      />
    </form>
    <template #footer>
      <NFlex justify="end">
        <AppButton type="button" @click="settleConfirmation(false)"
          >取消</AppButton
        ><AppButton type="submit" form="confirmation-form" class="primary"
          >确认</AppButton
        >
      </NFlex>
    </template></AppDialog
  >
</template>

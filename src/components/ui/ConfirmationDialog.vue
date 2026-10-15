<script setup lang="ts">
import AppDialog from "./AppDialog.vue";
import { currentConfirmation, settleConfirmation } from "./confirmation";
</script>
<template>
  <AppDialog
    :open="!!currentConfirmation"
    :title="currentConfirmation?.input === undefined ? '确认操作' : '输入信息'"
    @update:open="(value) => !value && settleConfirmation(false)"
    ><form
      v-if="currentConfirmation"
      @submit.prevent="settleConfirmation(true)"
    >
      <p class="confirmation-message">{{ currentConfirmation.message }}</p>
      <input
        v-if="currentConfirmation.input !== undefined"
        v-model="currentConfirmation.input"
        maxlength="120"
        autofocus
      />
      <footer class="dialog-actions">
        <button type="button" @click="settleConfirmation(false)">取消</button
        ><button class="primary">确认</button>
      </footer>
    </form></AppDialog
  >
</template>

<script setup lang="ts">
import { ref } from "vue";
import { NIcon } from "naive-ui";
import { IconX } from "@tabler/icons-vue";
import AppButton from "@/components/ui/AppButton.vue";
import AppInput from "@/components/ui/AppInput.vue";
import { t } from "@/i18n";
import { terminalRegistry } from "./registry";

const props = defineProps<{ open: boolean; terminalId?: string }>();
const emit = defineEmits<{ close: [] }>();
const query = ref("");

function find(previous = false) {
  if (props.terminalId)
    terminalRegistry.find(props.terminalId, query.value, previous);
}
</script>

<template>
  <div v-if="open && terminalId" class="find-bar">
    <AppInput
      v-model="query"
      autofocus
      :aria-label="t('terminal.search.label')"
      :placeholder="t('terminal.search.placeholder')"
      @keydown.enter.prevent="find($event.shiftKey)"
      @keydown.esc="emit('close')"
    />
    <AppButton @click="find(true)">
      {{ t("common.actions.previous") }}
    </AppButton>
    <AppButton @click="find()">
      {{ t("common.actions.next") }}
    </AppButton>
    <AppButton
      class="icon-button"
      :aria-label="t('terminal.search.close')"
      @click="emit('close')"
    >
      <NIcon :size="16" aria-hidden="true"><IconX /></NIcon>
    </AppButton>
  </div>
</template>

<style scoped>
.find-bar {
  display: flex;
  gap: 6px;
  justify-content: flex-end;
  padding: 8px;
  border-bottom: 1px solid var(--border);
}
.find-bar .ui-input {
  max-width: 300px;
}
</style>

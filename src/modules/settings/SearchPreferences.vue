<script setup lang="ts">
import { t } from "../../i18n";
import { NCheckbox, NCheckboxGroup, NSpace, NSwitch, NText } from "naive-ui";
import { computed, ref } from "vue";
import { useSettings } from "./store";
import type { HostSearchSettings, HostSearchField } from "./model";
import { errorMessage } from "../../services/desktop/client";
defineProps<{ compact?: boolean }>();
const store = useSettings();
const busy = ref(false),
  error = ref("");
const fields = computed<{ value: HostSearchField; label: string }[]>(() => [
  { value: "name", label: t("common.fields.name") },
  { value: "address", label: t("settings.search.address") },
  { value: "username", label: t("common.fields.username") },
  { value: "tags", label: t("common.fields.tags") },
  { value: "group", label: t("common.fields.group") },
]);
async function update(patch: Partial<HostSearchSettings>) {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    await store.setHostSearch(patch);
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
function select(values: (string | number)[]) {
  void update({
    fields: fields.value
      .filter((field) => field.value === "name" || values.includes(field.value))
      .map((field) => field.value),
  });
}
</script>
<template>
  <NSpace vertical :size="12">
    <NSpace v-if="!compact" align="center">
      <NText>{{ t("settings.search.pinyin") }}</NText>
      <NSwitch
        :value="store.value.hostSearch.pinyin"
        :disabled="busy"
        :aria-label="t('settings.search.pinyin')"
        @update:value="update({ pinyin: $event })"
      />
    </NSpace>
    <NCheckboxGroup
      :value="store.value.hostSearch.fields"
      :disabled="busy"
      @update:value="select"
    >
      <NSpace>
        <NCheckbox
          v-for="field in fields"
          :key="field.value"
          :value="field.value"
          :disabled="busy || field.value === 'name'"
        >
          {{ field.label }}
        </NCheckbox>
      </NSpace>
    </NCheckboxGroup>
    <NText v-if="!compact" depth="3">
      {{ t("settings.search.description") }}
    </NText>
    <NText v-if="error" type="error" role="alert">{{ error }}</NText>
  </NSpace>
</template>

<script setup lang="ts">
import { NCheckbox, NCheckboxGroup, NSpace, NSwitch, NText } from "naive-ui";
import { ref } from "vue";
import { useSettings } from "./store";
import type { HostSearchSettings, HostSearchField } from "./model";
import { errorMessage } from "../../services/desktop/client";
defineProps<{ compact?: boolean }>();
const store = useSettings();
const busy = ref(false),
  error = ref("");
const fields: { value: HostSearchField; label: string }[] = [
  { value: "name", label: "名称" },
  { value: "address", label: "地址" },
  { value: "username", label: "用户名" },
  { value: "tags", label: "标签" },
  { value: "group", label: "分组" },
];
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
    fields: fields
      .filter((field) => field.value === "name" || values.includes(field.value))
      .map((field) => field.value),
  });
}
</script>
<template>
  <NSpace vertical :size="12">
    <NSpace v-if="!compact" align="center">
      <NText>拼音模糊搜索</NText>
      <NSwitch
        :value="store.value.hostSearch.pinyin"
        :disabled="busy"
        aria-label="拼音模糊搜索"
        @update:value="update({ pinyin: $event })"
      />
    </NSpace>
    <NCheckboxGroup
      :value="store.value.hostSearch.fields"
      :disabled="busy"
      @update:value="select"
    >
      <NSpace
        ><NCheckbox
          v-for="field in fields"
          :key="field.value"
          :value="field.value"
          :disabled="busy || field.value === 'name'"
          >{{ field.label }}</NCheckbox
        ></NSpace
      >
    </NCheckboxGroup>
    <NText v-if="!compact" depth="3"
      >搜索设置修改后立即保存；名称始终参与搜索。</NText
    >
    <NText v-if="error" type="error" role="alert">{{ error }}</NText>
  </NSpace>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NForm,
  NFormItem,
  NInput,
  NSpace,
  NText,
  type FormInst,
  type FormRules,
} from "naive-ui";
import AppDialog from "../../components/ui/AppDialog.vue";
import { errorMessage } from "../../services/desktop/client";
import { requiredText, validateForm } from "../../services/forms/rules";
import type { SavedWorkspace } from "../../services/desktop/types";
import { useHosts } from "../hosts/store";
import { useWorkspace } from "./store";
const props = defineProps<{ groupId: string }>();
const emit = defineEmits<{ close: []; "after-leave": [] }>();
const hosts = useHosts();
const workspace = useWorkspace();
const model = reactive({ name: "" });
const form = ref<FormInst>();
const error = ref("");
const busy = ref(false);
const snapshot = ref<SavedWorkspace>();
const existing = computed(() =>
  hosts.hosts.find((h) => h.name === model.name.trim()),
);
const rules: FormRules = {
  name: [
    requiredText("名称"),
    {
      validator: () =>
        !existing.value ||
        existing.value.kind === "workspace" ||
        new Error("名称已被连接使用，只允许替换同名工作区"),
      trigger: ["input", "blur"],
    },
  ],
};
watch(
  () => props.groupId,
  (id) => {
    if (!id) return;
    error.value = "";
    snapshot.value = undefined;
    const group = workspace.groups.find((g) => g.id === id);
    model.name = group?.name ?? "Workspace";
    try {
      snapshot.value = workspace.snapshotWorkspace(id);
    } catch (e) {
      error.value = errorMessage(e);
    }
  },
);
async function save() {
  if (busy.value || !snapshot.value || !(await validateForm(form.value)))
    return;
  busy.value = true;
  error.value = "";
  try {
    const saved = await hosts.saveWorkspace({
      name: model.name.trim(),
      workspace: snapshot.value,
      expectedRevision: existing.value?.revision ?? null,
    });
    const group = workspace.groups.find((g) => g.id === props.groupId);
    if (group) {
      group.name = saved.name;
      group.savedProfileId = saved.id;
    }
    emit("close");
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <AppDialog
    :open="!!groupId"
    title="保存工作区"
    :locked="busy"
    @update:open="emit('close')"
    @after-leave="emit('after-leave')"
  >
    <NForm ref="form" :model="model" :rules="rules" @submit.prevent="save">
      <NFormItem label="名称" path="name">
        <NInput
          v-model:value="model.name"
          :maxlength="128"
          :disabled="busy"
          @keydown.enter.prevent="save"
        />
      </NFormItem>
    </NForm>
    <NText depth="3">
      保存布局与连接引用。保存后无法编辑，可重新保存替换同名工作区。
    </NText>
    <NAlert v-if="existing?.kind === 'workspace'" type="warning">
      将替换工作区“{{ existing.name }}”的布局与连接引用。
    </NAlert>
    <NAlert v-if="error" type="error">{{ error }}</NAlert>
    <template #footer>
      <NSpace justify="end">
        <NButton :disabled="busy" @click="emit('close')">取消</NButton>
        <NButton
          type="primary"
          :loading="busy"
          :disabled="!snapshot"
          @click="save"
        >
          {{ existing?.kind === "workspace" ? "替换工作区" : "保存" }}
        </NButton>
      </NSpace>
    </template>
  </AppDialog>
</template>

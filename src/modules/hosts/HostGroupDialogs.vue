<script setup lang="ts">
import { computed, nextTick, reactive, ref } from "vue";
import {
  NButton,
  NForm,
  NFormItem,
  NInput,
  NRadio,
  NRadioGroup,
  NSelect,
  NSpace,
  NText,
  type FormInst,
} from "naive-ui";
import { t } from "@/i18n";
import AppDialog from "@/components/ui/AppDialog.vue";
import { confirmAction } from "@/components/ui/confirmation";
import { requiredText, validateForm } from "@/services/forms/rules";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { HostGroup } from "./model";
import {
  descendantIds,
  groupChoices,
  groupPath,
  nextGroupOrder,
} from "./groups";
import { useHosts } from "./store";

const props = defineProps<{ selectedId: string }>();
const emit = defineEmits<{
  saved: [id: string];
  select: [id: string];
  error: [message: string];
}>();
const store = useHosts();
const editor = ref(false);
const editing = ref<HostGroup>();
const name = ref("");
const parent = ref<string | null>(null);
const groupModel = reactive({ name, parent });
const dialogError = ref("");
const deleteTarget = ref<HostGroup>();
const deleteMode = ref("move");
const deleteError = ref("");
const deletePreview = ref({ hosts: 0, groups: 0, direct: 0, parent: "" });
const parentOptions = computed(() => [
  { label: t("hosts.groups.topLevel"), value: "" },
  ...groupChoices(
    store.groups,
    editing.value ? descendantIds(store.groups, editing.value.id) : undefined,
  ),
]);
function editGroup(group?: HostGroup, parentId: string | null = null) {
  editing.value = group ? { ...group } : undefined;
  name.value = group?.name ?? "";
  parent.value = group?.parentId ?? parentId;
  dialogError.value = "";
  editor.value = true;
}
function prepareDelete(group: HostGroup) {
  const ids = descendantIds(store.groups, group.id);
  deletePreview.value = {
    groups: ids.size - 1,
    hosts: store.hosts.filter((host) => host.groupId && ids.has(host.groupId))
      .length,
    direct: store.hosts.filter((host) => host.groupId === group.id).length,
    parent: group.parentId ? groupPath(store.groups, group.parentId) : "",
  };
  deleteMode.value = "move";
  deleteError.value = "";
  deleteTarget.value = { ...group };
}
const form = ref<FormInst>();
const groupRules = computed(() => ({
  name: requiredText("hosts.groups.name"),
}));
async function saveGroup() {
  if (store.managementBusy || !(await validateForm(form.value))) return;
  store.managementBusy = true;
  dialogError.value = "";
  try {
    const saved = await store.saveGroup(
      {
        id: editing.value?.id ?? null,
        name: name.value.trim(),
        parentId: parent.value || null,
        sortOrder:
          editing.value?.sortOrder ??
          nextGroupOrder(store.groups, parent.value || null),
      },
      editing.value?.revision ?? null,
    );
    emit("saved", saved.id);
    editor.value = false;
  } catch (error) {
    dialogError.value = errorMessage(error);
  } finally {
    store.managementBusy = false;
  }
}
async function deleteGroup() {
  const target = deleteTarget.value;
  if (!target || store.managementBusy) return;
  store.managementBusy = true;
  deleteError.value = "";
  const deleteHosts = deleteMode.value === "delete";
  try {
    if (deleteHosts) {
      // 先关闭选择弹窗，再显示独立的最终确认，避免两个焦点陷阱重叠。
      deleteTarget.value = undefined;
      await nextTick();
      if (
        !(await confirmAction(
          t("hosts.groups.confirmDeleteAll", {
            name: target.name,
            groups: deletePreview.value.groups,
            hosts: deletePreview.value.hosts,
          }),
        ))
      )
        return;
    }
    await desktop.deleteGroup(target.id, target.revision, deleteHosts);
    deleteTarget.value = undefined;
    if (
      (deleteHosts &&
        descendantIds(store.groups, target.id).has(props.selectedId)) ||
      props.selectedId === target.id
    )
      emit("select", target.parentId ?? "ungrouped");
    // 删除已经提交，重读失败时明确提示，避免让用户重复执行删除。
    try {
      await store.load();
    } catch (error) {
      emit(
        "error",
        t("hosts.groups.refreshAfterDeleteFailed", {
          value1: errorMessage(error),
        }),
      );
    }
  } catch (error) {
    if (deleteTarget.value) deleteError.value = errorMessage(error);
    else emit("error", errorMessage(error));
  } finally {
    store.managementBusy = false;
  }
}

defineExpose({ edit: editGroup, remove: prepareDelete });
</script>

<template>
  <AppDialog
    v-model:open="editor"
    :title="editing ? t('hosts.groups.edit') : t('hosts.groups.create')"
    :locked="store.managementBusy"
  >
    <NForm
      ref="form"
      :model="groupModel"
      :rules="groupRules"
      @submit.prevent="saveGroup"
    >
      <NFormItem :label="t('hosts.groups.name')" path="name">
        <NInput
          v-model:value="groupModel.name"
          autofocus
          maxlength="128"
          @keydown.enter.prevent="saveGroup"
        />
      </NFormItem>
      <NFormItem :label="t('hosts.groups.parent')">
        <NSelect
          :value="parent ?? ''"
          :options="parentOptions"
          filterable
          @update:value="parent = $event || null"
        />
      </NFormItem>
      <NText v-if="dialogError" type="error" role="alert">
        {{ dialogError }}
      </NText>
    </NForm>
    <template #footer>
      <NSpace justify="end">
        <NButton :disabled="store.managementBusy" @click="editor = false">
          {{ t("common.actions.cancel") }}
        </NButton>
        <NButton
          type="primary"
          :loading="store.managementBusy"
          @click="saveGroup"
        >
          {{ t("common.actions.save") }}
        </NButton>
      </NSpace>
    </template>
  </AppDialog>
  <AppDialog
    :open="!!deleteTarget"
    :title="t('hosts.groups.delete')"
    :locked="store.managementBusy"
    @update:open="
      (value) => {
        if (!value) deleteTarget = undefined;
      }
    "
  >
    <p>
      {{
        t("hosts.groups.deletePrompt", {
          name: deleteTarget?.name ?? "",
        })
      }}
    </p>
    <NRadioGroup v-model:value="deleteMode" name="delete-group-mode">
      <NSpace vertical>
        <NRadio value="move">{{ t("hosts.groups.keepContents") }}</NRadio>
        <NText depth="3">
          {{
            t("hosts.groups.moveContentsDescription", {
              direct: deletePreview.direct,
              parent:
                deletePreview.parent || t("hosts.groups.topLevelDestination"),
            })
          }}
        </NText>
        <NRadio value="delete">{{ t("hosts.groups.deleteContents") }}</NRadio>
        <NText depth="3">
          {{
            t("hosts.groups.deleteContentsDescription", {
              groups: deletePreview.groups,
              hosts: deletePreview.hosts,
            })
          }}
        </NText>
      </NSpace>
    </NRadioGroup>
    <NText v-if="deleteError" type="error" role="alert">
      {{ deleteError }}
    </NText>
    <template #footer>
      <NSpace justify="end">
        <NButton
          :disabled="store.managementBusy"
          @click="deleteTarget = undefined"
        >
          {{ t("common.actions.cancel") }}
        </NButton>
        <NButton
          :type="deleteMode === 'delete' ? 'error' : 'primary'"
          :loading="store.managementBusy"
          @click="deleteGroup"
        >
          {{
            deleteMode === "delete"
              ? t("common.actions.continueMenu")
              : t("hosts.groups.deleteAndMove")
          }}
        </NButton>
      </NSpace>
    </template>
  </AppDialog>
</template>

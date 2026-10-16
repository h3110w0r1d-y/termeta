<script setup lang="ts">
import { closeContextMenuOnOutside } from "../../components/ui/contextMenu";
import { computed, h, nextTick, ref, watch } from "vue";
import {
  NButton,
  NDropdown,
  NForm,
  type FormInst,
  NFormItem,
  NInput,
  NRadio,
  NRadioGroup,
  NScrollbar,
  NSelect,
  NSpace,
  NText,
  NTree,
  type TreeOption,
} from "naive-ui";
import { requiredText, validateForm } from "../../services/forms/rules";
import AppDialog from "../../components/ui/AppDialog.vue";
import { confirmAction } from "../../components/ui/confirmation";
import { desktop, errorMessage } from "../../services/desktop/client";
import type { HostGroup } from "../../services/desktop/types";
import { descendantIds, groupChoices, groupPath } from "./groups";
import { useHosts } from "./store";
import { useSettings } from "../settings/store";
import { useHostGroupDrag } from "./useHostGroupDrag";

const props = defineProps<{ modelValue: string; active: boolean }>();
const emit = defineEmits<{
  "update:modelValue": [string];
  error: [string];
}>();
const store = useHosts();
const panel = ref<HTMLElement>();
const previewLayer = ref<HTMLElement>();
const settings = useSettings();
const expanded = ref<string[]>([...settings.value.hostsView.expandedGroupIds]);
let pendingExpansionSaves = 0;
let expansionGeneration = 0;
async function setExpanded(ids: string[]) {
  const next = [...new Set(ids)].filter((id) =>
    store.groups.some((group) => group.id === id),
  );
  if (JSON.stringify(next) === JSON.stringify(expanded.value)) return;
  expanded.value = next;
  const generation = ++expansionGeneration;
  pendingExpansionSaves++;
  try {
    await settings.setHostsView({ expandedGroupIds: next });
  } catch (error) {
    if (generation === expansionGeneration)
      expanded.value = [...settings.value.hostsView.expandedGroupIds];
    emit("error", `分组展开状态保存失败：${errorMessage(error)}`);
  } finally {
    pendingExpansionSaves--;
  }
}
watch(
  () => settings.value.hostsView.expandedGroupIds,
  (ids) => {
    if (!pendingExpansionSaves) expanded.value = [...ids];
  },
);
const menu = ref({ show: false, id: "all", x: 0, y: 0 });
const editor = ref(false);
const editing = ref<HostGroup>();
const name = ref("");
const parent = ref<string | null>(null);
const dialogError = ref("");
const deleteTarget = ref<HostGroup>();
const deleteMode = ref("move");
const deleteError = ref("");
const deletePreview = ref({ hosts: 0, groups: 0, direct: 0, parent: "" });
const tree = computed<TreeOption[]>(() => {
  const ordered = [...store.groups].sort(
    (a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name),
  );
  const counts = new Map<string | null, number>();
  for (const host of store.hosts)
    counts.set(host.groupId, (counts.get(host.groupId) ?? 0) + 1);
  const build = (
    parentId: string | null,
    seen = new Set<string>(),
  ): TreeOption[] =>
    ordered
      .filter((group) => group.parentId === parentId && !seen.has(group.id))
      .map((group) => {
        const children = build(group.id, new Set([...seen, group.id]));
        return {
          key: group.id,
          label: group.name,
          count:
            (counts.get(group.id) ?? 0) +
            children.reduce((total, child) => total + Number(child.count), 0),
          children,
        };
      });
  return [
    { key: "all", label: "全部主机", count: store.hosts.length },
    {
      key: "ungrouped",
      label: "未分组",
      count: store.hosts.filter((host) => !host.groupId).length,
    },
    ...build(null),
  ];
});
const parentOptions = computed(() => [
  { label: "顶层", value: "" },
  ...groupChoices(
    store.groups,
    editing.value ? descendantIds(store.groups, editing.value.id) : undefined,
  ),
]);
const menuOptions = computed(() => {
  const real = store.groups.some((group) => group.id === menu.value.id);
  return [
    {
      label: real ? "创建子分组" : "创建分组",
      key: "create",
      disabled: store.managementBusy,
    },
    {
      label: "编辑分组 / 修改父组",
      key: "edit",
      disabled: !real || store.managementBusy,
    },
    {
      label: "删除分组…",
      key: "delete",
      disabled: !real || store.managementBusy,
    },
  ];
});
function openMenu(event: MouseEvent | KeyboardEvent, id: string) {
  if (store.managementBusy) return;
  event.preventDefault();
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  menu.value = {
    show: true,
    id,
    x: event instanceof MouseEvent ? event.clientX : rect.left,
    y: event instanceof MouseEvent ? event.clientY : rect.bottom,
  };
}
function panelMenu(event: MouseEvent) {
  if (!(event.target as HTMLElement).closest("[data-host-group-target]"))
    openMenu(event, "all");
}
function editGroup(group?: HostGroup, parentId: string | null = null) {
  editing.value = group ? { ...group } : undefined;
  name.value = group?.name ?? "";
  parent.value = group?.parentId ?? parentId;
  dialogError.value = "";
  editor.value = true;
}
function selectMenu(key: string) {
  menu.value.show = false;
  const group = store.groups.find((item) => item.id === menu.value.id);
  if (key === "create") editGroup(undefined, group?.id ?? null);
  else if (key === "edit" && group) editGroup(group);
  else if (key === "delete" && group) {
    const ids = descendantIds(store.groups, group.id);
    deletePreview.value = {
      groups: ids.size - 1,
      hosts: store.hosts.filter((host) => host.groupId && ids.has(host.groupId))
        .length,
      direct: store.hosts.filter((host) => host.groupId === group.id).length,
      parent: group.parentId
        ? groupPath(store.groups, group.parentId)
        : "顶层（主机进入未分组）",
    };
    deleteMode.value = "move";
    deleteError.value = "";
    deleteTarget.value = { ...group };
  }
}
function reveal(id: string | null) {
  const next = new Set(expanded.value);
  const seen = new Set<string>();
  while (id && !seen.has(id)) {
    seen.add(id);
    next.add(id);
    id = store.groups.find((group) => group.id === id)?.parentId ?? null;
  }
  void setExpanded([...next]);
}
const form = ref<FormInst>();
const groupRules = { name: requiredText("分组名称") };
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
        sortOrder: editing.value?.sortOrder ?? nextOrder(parent.value || null),
      },
      editing.value?.revision ?? null,
    );
    reveal(saved.id);
    emit("update:modelValue", saved.id);
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
          `再次确认：永久删除“${target.name}”、${deletePreview.value.groups} 个子分组及其中 ${deletePreview.value.hosts} 台主机？主机专属密码会一并清理，无法撤销；现有会话和共享凭据不受影响。`,
        ))
      )
        return;
    }
    await desktop.deleteGroup(target.id, target.revision, deleteHosts);
    deleteTarget.value = undefined;
    if (
      (deleteHosts &&
        descendantIds(store.groups, target.id).has(props.modelValue)) ||
      props.modelValue === target.id
    )
      emit("update:modelValue", target.parentId ?? "ungrouped");
    // 删除已经提交，重读失败时明确提示，避免让用户重复执行删除。
    try {
      await store.load();
    } catch (error) {
      emit(
        "error",
        `分组已删除，但列表刷新失败：${errorMessage(error)}。请刷新列表。`,
      );
    }
  } catch (error) {
    if (deleteTarget.value) deleteError.value = errorMessage(error);
    else emit("error", errorMessage(error));
  } finally {
    store.managementBusy = false;
  }
}
function nextOrder(parentId: string | null) {
  return (
    Math.max(
      0,
      ...store.groups
        .filter((group) => group.parentId === parentId)
        .map((group) => group.sortOrder),
    ) + 1
  );
}

const { drag, startDrag, startHostDrag, cancelDrag } = useHostGroupDrag({
  panel,
  previewLayer,
  reveal,
  nextOrder,
  closeMenu: () => {
    menu.value.show = false;
  },
  onError: (message) => emit("error", message),
});
function nodeProps({ option }: { option: TreeOption }) {
  const id = String(option.key);
  return {
    "data-host-group-target": id,
    class:
      drag.value?.moved && drag.value.target === id
        ? "group-drop-target"
        : undefined,
    onContextmenu: (event: MouseEvent) => openMenu(event, id),
    onPointerdown: (event: PointerEvent) => {
      if (!["all", "ungrouped"].includes(id))
        startDrag(event, "group", id, String(option.label));
    },
  };
}
function treeKey(event: KeyboardEvent) {
  if (store.managementBusy) return;
  if (
    event.key !== "ContextMenu" &&
    !(event.shiftKey && event.key === "F10") &&
    event.key !== "F2"
  )
    return;
  // NTree 的键盘焦点在根容器，操作库当前高亮行；没有待选行时使用选中分组。
  const row = (event.currentTarget as HTMLElement).querySelector<HTMLElement>(
    ".n-tree-node--pending[data-host-group-target]",
  );
  const id = row?.dataset.hostGroupTarget ?? props.modelValue;
  if (event.key === "F2") {
    const group = store.groups.find((group) => group.id === id);
    if (group) {
      event.preventDefault();
      event.stopPropagation();
      editGroup(group);
    }
  } else {
    openMenu(event, id);
    if (row) {
      const rect = row.getBoundingClientRect();
      menu.value.x = rect.left;
      menu.value.y = rect.bottom;
    }
  }
}
function folderIcon() {
  return h(
    "svg",
    {
      width: 18,
      height: 18,
      viewBox: "0 0 24 24",
      fill: "none",
      stroke: "currentColor",
      "stroke-width": 1.6,
      "aria-hidden": true,
    },
    [h("path", { d: "M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v10H3Z" })],
  );
}
watch(
  () => props.active,
  (active) => {
    if (!active) {
      menu.value.show = false;
      cancelDrag();
    }
  },
);
watch(
  () => store.groups.map((group) => group.id),
  () => {
    void setExpanded(expanded.value);
    if (
      !["all", "ungrouped"].includes(props.modelValue) &&
      !store.groups.some((group) => group.id === props.modelValue)
    )
      emit("update:modelValue", "all");
  },
  { immediate: true },
);
defineExpose({ startHostDrag });
</script>

<template>
  <Teleport to="body">
    <div
      ref="previewLayer"
      class="host-drag-preview"
      aria-hidden="true"
      inert
    />
  </Teleport>
  <aside
    ref="panel"
    class="host-groups"
    aria-label="主机分组"
    @contextmenu="panelMenu"
  >
    <div class="groups-heading">
      <NText strong>分组</NText>
      <NButton
        size="small"
        :disabled="store.managementBusy"
        @click="
          editGroup(
            undefined,
            store.groups.some((g) => g.id === modelValue) ? modelValue : null,
          )
        "
      >
        新建分组
      </NButton>
    </div>
    <NScrollbar class="groups-scroll">
      <NTree
        block-line
        selectable
        :cancelable="false"
        :data="tree"
        :selected-keys="[modelValue]"
        :expanded-keys="expanded"
        :node-props="nodeProps"
        :render-prefix="folderIcon"
        @keydown.capture="treeKey"
        :render-suffix="
          ({ option }) => h(NText, { depth: 3 }, () => `(${option.count})`)
        "
        @update:expanded-keys="setExpanded($event.map(String))"
        @update:selected-keys="
          (keys) => {
            if (keys.length && !store.managementBusy)
              emit('update:modelValue', String(keys[0]));
          }
        "
      />
    </NScrollbar>
    <NDropdown
      :show="menu.show"
      :x="menu.x"
      :y="menu.y"
      :options="menuOptions"
      trigger="manual"
      placement="bottom-start"
      :menu-props="() => ({ 'data-ui-popup-open': menu.show ? '' : undefined })"
      display-directive="show"
      @select="selectMenu"
      @clickoutside="closeContextMenuOnOutside($event, menu)"
      @update:show="menu.show = $event"
    />
    <AppDialog
      v-model:open="editor"
      :title="editing ? '编辑分组' : '创建分组'"
      :locked="store.managementBusy"
    >
      <NForm
        ref="form"
        :model="{ name, parent }"
        :rules="groupRules"
        @submit.prevent="saveGroup"
      >
        <NFormItem label="分组名称" path="name">
          <NInput
            v-model:value="name"
            autofocus
            maxlength="128"
            @keydown.enter.prevent="saveGroup"
          />
        </NFormItem>
        <NFormItem label="父分组">
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
            取消
          </NButton>
          <NButton
            type="primary"
            :loading="store.managementBusy"
            @click="saveGroup"
          >
            保存
          </NButton>
        </NSpace>
      </template>
    </AppDialog>
    <AppDialog
      :open="!!deleteTarget"
      title="删除分组"
      :locked="store.managementBusy"
      @update:open="
        (value) => {
          if (!value) deleteTarget = undefined;
        }
      "
    >
      <p>删除“{{ deleteTarget?.name }}”，请选择如何处理其中内容：</p>
      <NRadioGroup v-model:value="deleteMode" name="delete-group-mode">
        <NSpace vertical>
          <NRadio value="move">保留主机与子分组，移到父级</NRadio>
          <NText depth="3">
            {{ deletePreview.direct }} 台直属主机和直属子分组移到
            {{ deletePreview.parent }}，子分组内的内容保持原层级。
          </NText>
          <NRadio value="delete">一并删除所有子分组和主机</NRadio>
          <NText depth="3">
            删除 {{ deletePreview.groups }} 个子分组和其中全部
            {{ deletePreview.hosts }} 台主机，需要再次确认。
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
            取消
          </NButton>
          <NButton
            :type="deleteMode === 'delete' ? 'error' : 'primary'"
            :loading="store.managementBusy"
            @click="deleteGroup"
          >
            {{ deleteMode === "delete" ? "继续…" : "删除分组并移动内容" }}
          </NButton>
        </NSpace>
      </template>
    </AppDialog>
  </aside>
</template>

<style scoped>
.host-drag-preview {
  display: none;
  position: fixed;
  left: 0;
  top: 0;
  z-index: 3000;
  pointer-events: none;
  user-select: none;
}
.host-drag-preview :deep(.host-drag-hint) {
  margin-top: 8px;
  padding: 4px 8px;
  width: max-content;
  max-width: min(320px, 80vw);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  background: var(--surface);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-size: 12px;
}

.host-groups {
  display: flex;
  flex-direction: column;
  width: 240px;
  flex-shrink: 0;
  min-height: 0;
  border-right: 1px solid var(--border);
  padding-right: 16px;
}
.groups-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.groups-scroll {
  flex: 1;
  min-height: 0;
}
.groups-help,
.drag-status {
  padding-top: 12px;
  font-size: 12px;
}
:deep(.group-drop-target) {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
:deep(.n-tree-node-content) {
  min-width: 0;
}
:deep(.n-tree-node-content__text) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
@media (max-width: 1000px) {
  .host-groups {
    width: 190px;
    padding-right: 12px;
  }
}
</style>

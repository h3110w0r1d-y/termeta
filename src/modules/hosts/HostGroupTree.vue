<script setup lang="ts">
import { t } from "@/i18n";
import ScrollFade from "@/components/ui/ScrollFade.vue";
import { closeContextMenuOnOutside } from "@/components/ui/contextMenu";
import { computed, h, ref, watch } from "vue";
import {
  NButton,
  NDropdown,
  NScrollbar,
  NSwitch,
  NText,
  NTree,
  type TreeOption,
} from "naive-ui";
import { errorMessage } from "@/services/desktop/client";
import HostProfileIcon from "./HostProfileIcon.vue";
import HostGroupDialogs from "./HostGroupDialogs.vue";
import type { HostGroup, HostProfile } from "@/modules/hosts/model";
import { nextGroupOrder } from "./groups";
import { useHosts } from "./store";
import { useSettings } from "../settings/store";
import { useHostGroupDrag } from "./useHostGroupDrag";
import { IconFolder } from "@tabler/icons-vue";

const props = defineProps<{
  modelValue: string;
  active: boolean;
  selectedHostIds: string[];
}>();
const emit = defineEmits<{
  "update:modelValue": [string];
  error: [string];
  connect: [HostProfile];
  connectGroup: [string];
  hostMenu: [MouseEvent | KeyboardEvent, HostProfile];
  editHost: [HostProfile];
  hostDragStart: [string[]];
}>();
const store = useHosts();
const panel = ref<HTMLElement>();
const previewLayer = ref<HTMLElement>();
const settings = useSettings();
const showHosts = computed(() => settings.value.hostsView.showHosts);
const savingShowHosts = ref(false);
async function setShowHosts(value: boolean) {
  if (savingShowHosts.value) return;
  savingShowHosts.value = true;
  cancelDrag();
  try {
    await settings.setHostsView({ showHosts: value });
  } catch (error) {
    emit(
      "error",
      t("hosts.errors.sidebarSave", { value1: errorMessage(error) }),
    );
  } finally {
    savingShowHosts.value = false;
  }
}
const expanded = ref<string[]>([...settings.value.hostsView.expandedGroupIds]);
let pendingExpansionSaves = 0;
let expansionGeneration = 0;
async function setExpanded(ids: string[]) {
  const next = [...new Set(ids)].filter(
    (id) => id === "ungrouped" || store.groups.some((group) => group.id === id),
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
    emit(
      "error",
      t("hosts.errors.expansionSave", { value1: errorMessage(error) }),
    );
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
const dialogs = ref<InstanceType<typeof HostGroupDialogs>>();
function hostFromOption(option: TreeOption) {
  return option.hostId
    ? store.hosts.find((host) => host.id === option.hostId)
    : undefined;
}
const tree = computed<TreeOption[]>(() => {
  const ordered = [...store.groups].sort(
    (a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name),
  );
  const counts = new Map<string | null, number>();
  for (const host of store.hosts)
    counts.set(host.groupId, (counts.get(host.groupId) ?? 0) + 1);
  const hostNodes = (groupId: string | null): TreeOption[] =>
    showHosts.value
      ? store.orderedHosts
          .filter((host) => host.groupId === groupId)
          .map((host) => ({
            key: `host:${host.id}`,
            label: host.name,
            hostId: host.id,
            isLeaf: true,
          }))
      : [];
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
          // 先汇总子分组计数，再加入主机叶节点，避免主机重复计数。
          children: [...children, ...hostNodes(group.id)],
          isLeaf:
            children.length === 0 &&
            (!showHosts.value || !counts.get(group.id)),
        };
      });
  return [
    {
      key: "all",
      label: t("hosts.filters.all"),
      count: store.hosts.length,
      isLeaf: true,
    },
    {
      key: "ungrouped",
      label: t("hosts.groups.ungrouped"),
      count: store.hosts.filter((host) => !host.groupId).length,
      children: hostNodes(null),
      isLeaf: !showHosts.value || !counts.get(null),
    },
    ...build(null),
  ];
});
const menuOptions = computed(() => {
  const real = store.groups.some((group) => group.id === menu.value.id);
  return [
    {
      label: real ? t("hosts.groups.newChild") : t("hosts.groups.create"),
      key: "create",
      disabled: store.managementBusy,
    },
    {
      label: t("hosts.groups.editParent"),
      key: "edit",
      disabled: !real || store.managementBusy,
    },
    {
      label: t("hosts.groups.deleteMenu"),
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
  if (
    !(event.target as HTMLElement).closest(
      "[data-host-group-target], [data-host-id], button, [role=switch]",
    )
  )
    openMenu(event, "all");
}
function editGroup(group?: HostGroup, parentId: string | null = null) {
  dialogs.value?.edit(group, parentId);
}
function savedGroup(id: string) {
  reveal(id);
  emit("update:modelValue", id);
}
function selectMenu(key: string) {
  menu.value.show = false;
  const group = store.groups.find((item) => item.id === menu.value.id);
  if (key === "create") editGroup(undefined, group?.id ?? null);
  else if (key === "edit" && group) editGroup(group);
  else if (key === "delete" && group) dialogs.value?.remove(group);
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
function nextOrder(parentId: string | null) {
  return nextGroupOrder(store.groups, parentId);
}

const { drag, startDrag, startHostDrag, cancelDrag } = useHostGroupDrag({
  panel,
  previewLayer,
  selectedHostIds: () => props.selectedHostIds,
  onHostDragStart: (ids) => emit("hostDragStart", ids),
  reveal,
  nextOrder,
  closeMenu: () => {
    menu.value.show = false;
  },
  onError: (message) => emit("error", message),
});
function nodeProps({ option }: { option: TreeOption }) {
  const host = hostFromOption(option);
  if (host) {
    return {
      "data-host-id": host.id,
      // 主机叶节点不作为分组落点，拖动仍可由叶节点发起。
      onContextmenu: (event: MouseEvent) => {
        event.stopPropagation();
        menu.value.show = false;
        emit("hostMenu", event, host);
      },
      onPointerdown: (event: PointerEvent) => startHostDrag(event, host),
      onDblclick: () => emit("connect", host),
    };
  }
  const id = String(option.key);
  return {
    "data-host-group-target": id,
    class:
      drag.value?.moved && drag.value.target === id
        ? "group-drop-target"
        : undefined,
    onContextmenu: (event: MouseEvent) => openMenu(event, id),
    onDblclick: (event: MouseEvent) => {
      // 展开箭头只控制树展开；双击分组正文才发起连接。
      if ((event.target as HTMLElement).closest(".n-tree-node-switcher"))
        return;
      if (props.active && !store.managementBusy && !drag.value?.moved) {
        event.stopPropagation();
        cancelDrag();
        menu.value.show = false;
        emit("connectGroup", id);
      }
    },
    onPointerdown: (event: PointerEvent) => {
      if (!["all", "ungrouped"].includes(id))
        startDrag(event, "group", id, String(option.label));
    },
  };
}
function treeKey(event: KeyboardEvent) {
  if (!props.active || store.managementBusy || event.isComposing) return;
  const pendingRow = (
    event.currentTarget as HTMLElement
  ).querySelector<HTMLElement>(".n-tree-node--pending");
  const host = store.hosts.find(
    (host) => host.id === pendingRow?.dataset.hostId,
  );
  if (host && event.key === "Enter") {
    event.preventDefault();
    event.stopPropagation();
    emit("connect", host);
    return;
  }
  if (!host && event.key === "Enter") {
    event.preventDefault();
    event.stopPropagation();
    menu.value.show = false;
    emit(
      "connectGroup",
      pendingRow?.dataset.hostGroupTarget ?? props.modelValue,
    );
    return;
  }
  if (
    event.key !== "ContextMenu" &&
    !(event.shiftKey && event.key === "F10") &&
    event.key !== "F2"
  )
    return;
  if (host) {
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "F2") emit("editHost", host);
    else {
      menu.value.show = false;
      emit("hostMenu", event, host);
    }
    return;
  }
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
function nodeIcon({ option }: { option: TreeOption }) {
  const host = hostFromOption(option);
  if (host) return h(HostProfileIcon, { host, size: 20 });
  return folderIcon();
}
function selectNode(keys: Array<string | number>) {
  if (!keys.length || store.managementBusy) return;
  const key = String(keys[0]);
  const host = store.hosts.find((host) => `host:${host.id}` === key);
  emit("update:modelValue", host ? (host.groupId ?? "ungrouped") : key);
}
function folderIcon() {
  return h(IconFolder, { size: 18, "aria-hidden": true });
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
defineExpose({ startHostDrag, cancelDrag });
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
    :aria-label="t('hosts.groups.title')"
    @contextmenu="panelMenu"
  >
    <div class="groups-heading">
      <NText strong>{{ t("common.fields.group") }}</NText>
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
        {{ t("hosts.groups.new") }}
      </NButton>
    </div>
    <div class="groups-hosts-toggle">
      <NText>{{ t("hosts.groups.showHosts") }}</NText>
      <NSwitch
        :value="showHosts"
        :disabled="savingShowHosts"
        size="small"
        :aria-label="t('hosts.groups.showHostsInSidebar')"
        @update:value="setShowHosts"
      />
    </div>
    <ScrollFade class="groups-scroll" background="var(--bg)" :size="20">
      <NScrollbar>
        <NTree
          selectable
          :cancelable="false"
          :data="tree"
          :show-line="true"
          :selected-keys="[modelValue]"
          :expanded-keys="expanded"
          :node-props="nodeProps"
          :render-prefix="nodeIcon"
          @keydown.capture="treeKey"
          :render-suffix="
            ({ option }) =>
              option.hostId
                ? null
                : h(NText, { depth: 3 }, () => `(${option.count})`)
          "
          @update:expanded-keys="setExpanded($event.map(String))"
          @update:selected-keys="selectNode"
        />
      </NScrollbar>
    </ScrollFade>
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
    <HostGroupDialogs
      ref="dialogs"
      :selected-id="modelValue"
      @saved="savedGroup"
      @select="emit('update:modelValue', $event)"
      @error="emit('error', $event)"
    />
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
.groups-hosts-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 12px;
}
.groups-scroll {
  flex: 1;
  min-height: 0;
}
:deep(.group-drop-target) {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
:deep(.n-tree-node-content) {
  width: 100%;
  min-width: 0;
}
/* 前缀只承载图形，避免继承文字行盒后产生基线留白。 */
:deep(.n-tree-node-content__prefix) {
  align-items: center;
  justify-content: center;
  line-height: 1;
}
:deep(.n-tree-node-content__prefix > svg) {
  display: block;
  flex-shrink: 0;
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

<script setup lang="ts">
import { confirmAction } from "../../components/ui/confirmation";
import { computed, nextTick, ref, watch } from "vue";
import {
  NForm,
  NFormItem,
  type FormInst,
  type FormRules,
  NAlert,
  NButton,
  NCard,
  NCheckbox,
  NDropdown,
  NEllipsis,
  NEmpty,
  NScrollbar,
  NSelect,
  NSpace,
  NTag,
  NText,
} from "naive-ui";
import { validateForm } from "../../services/forms/rules";
import AppDialog from "../../components/ui/AppDialog.vue";
import { errorMessage } from "../../services/desktop/client";
import type { HostProfile } from "../../services/desktop/types";
import HostGroupTree from "./HostGroupTree.vue";
import HostSearch from "./HostSearch.vue";
import { useSettings } from "../settings/store";
import { descendantIds, groupChoices, groupPath } from "./groups";
import { useHosts } from "./store";
const props = defineProps<{ active: boolean }>();
const emit = defineEmits<{
  connect: [HostProfile];
  edit: [HostProfile | undefined, boolean?, (string | null)?];
  settings: [];
  local: [];
}>();
const store = useHosts();
const settings = useSettings();
const search = ref<InstanceType<typeof HostSearch>>();
const groupTree = ref<InstanceType<typeof HostGroupTree>>();
const group = ref("all");
const includeChildren = computed(
  () => settings.value.hostsView.includeChildren,
);
const savingIncludeChildren = ref(false);
const error = ref("");
const selectedGroup = computed(() =>
  store.groups.find((item) => item.id === group.value),
);
const groupName = computed(() =>
  group.value === "all"
    ? "全部主机"
    : groupPath(store.groups, selectedGroup.value?.id ?? null),
);
const scope = computed(() =>
  includeChildren.value && selectedGroup.value
    ? descendantIds(store.groups, group.value)
    : new Set([group.value]),
);
const visible = computed(() =>
  store.hosts
    .filter(
      (host) =>
        group.value === "all" ||
        (group.value === "ungrouped"
          ? !host.groupId
          : !!host.groupId && scope.value.has(host.groupId)),
    )
    .sort(
      (a, b) =>
        Number(b.favorite) - Number(a.favorite) || a.name.localeCompare(b.name),
    ),
);
async function setIncludeChildren(value: boolean) {
  if (savingIncludeChildren.value) return;
  savingIncludeChildren.value = true;
  try {
    await settings.setHostsView({ includeChildren: value });
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    savingIncludeChildren.value = false;
  }
}
function createHost() {
  emit("edit", undefined, false, selectedGroup.value?.id ?? null);
}
const menu = ref({ show: false, hostId: "", x: 0, y: 0 });
const menuOptions = computed(() => [
  { label: "连接", key: "connect" },
  { label: "编辑", key: "edit", disabled: store.managementBusy },
  { label: "复制", key: "duplicate", disabled: store.managementBusy },
  { label: "移动到分组…", key: "move", disabled: store.managementBusy },
  { label: "删除", key: "delete", disabled: store.managementBusy },
]);
const movingHost = ref<HostProfile>();
const destination = ref("");
const moveError = ref("");
const moveOptions = computed(() => [
  { value: "", label: "未分组" },
  ...groupChoices(store.groups),
]);
function openMenu(event: MouseEvent | KeyboardEvent, host: HostProfile) {
  event.preventDefault();
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  menu.value = {
    show: true,
    hostId: host.id,
    x: event instanceof MouseEvent ? event.clientX : rect.left,
    y: event instanceof MouseEvent ? event.clientY : rect.bottom,
  };
}
function cardKey(event: KeyboardEvent, host: HostProfile) {
  if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10"))
    openMenu(event, host);
}
async function selectMenu(key: string) {
  menu.value.show = false;
  const host = store.hosts.find((item) => item.id === menu.value.hostId);
  if (!host) return;
  if (key === "connect") emit("connect", host);
  else if (!store.managementBusy) {
    if (key === "edit") emit("edit", host);
    else if (key === "duplicate") emit("edit", host, true);
    else if (key === "delete") await remove(host);
    else if (key === "move") {
      movingHost.value = { ...host };
      destination.value = host.groupId ?? "";
      moveError.value = "";
    }
  }
}
const moveForm = ref<FormInst>();
const moveRules: FormRules = {
  destination: {
    required: true,
    validator: () =>
      destination.value === "" ||
      store.groups.some((group) => group.id === destination.value) ||
      new Error("请选择有效分组"),
    trigger: "change",
  },
};
async function moveHost() {
  if (
    !movingHost.value ||
    store.managementBusy ||
    !(await validateForm(moveForm.value))
  )
    return;
  store.managementBusy = true;
  try {
    await store.moveHost(movingHost.value, destination.value || null);
    movingHost.value = undefined;
  } catch (e) {
    moveError.value = errorMessage(e);
  } finally {
    store.managementBusy = false;
  }
}
async function remove(host: HostProfile) {
  if (store.managementBusy) return;
  store.managementBusy = true;
  try {
    if (
      await confirmAction(
        `删除主机“${host.name}”？当前会话不受影响，共享凭据不会删除。`,
      )
    )
      await store.remove(host);
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    store.managementBusy = false;
  }
}
async function favorite(host: HostProfile) {
  if (store.managementBusy) return;
  store.managementBusy = true;
  try {
    await store.favorite(host);
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    store.managementBusy = false;
  }
}
watch(
  () => props.active,
  async (active) => {
    if (!active) {
      menu.value.show = false;
      return;
    }
    // 主机页保活，显隐更新后聚焦；保留选中分组和树展开状态。
    await nextTick();
    if (props.active) search.value?.focus();
  },
  { immediate: true, flush: "post" },
);
watch(group, () => {
  menu.value.show = false;
});
</script>
<template>
  <section class="hosts-page home-page">
    <HostSearch
      ref="search"
      :active="active"
      @connect="emit('connect', $event)"
    />
    <NSpace class="host-actions" justify="start">
      <NButton
        type="primary"
        :disabled="store.managementBusy"
        @click="createHost"
        >新建主机</NButton
      >
      <NButton @click="emit('local')">本地终端</NButton>
    </NSpace>
    <NAlert
      v-if="error"
      class="hosts-error"
      type="error"
      closable
      @close="error = ''"
      >{{ error }}</NAlert
    >
    <div class="host-layout">
      <HostGroupTree
        ref="groupTree"
        v-model="group"
        :active="active"
        @error="error = $event"
      />
      <section class="host-content" aria-label="分组主机">
        <div class="host-scope">
          <NEllipsis
            ><NText strong>{{ groupName }}</NText></NEllipsis
          >
          <NText depth="3">{{ visible.length }} 台主机</NText>
          <NCheckbox
            v-if="selectedGroup"
            :checked="includeChildren"
            :disabled="savingIncludeChildren"
            @update:checked="setIncludeChildren"
            >包含子分组</NCheckbox
          >
        </div>
        <NScrollbar class="host-list-scroll">
          <NEmpty
            v-if="!visible.length"
            class="hosts-empty"
            description="此分组暂无主机，可新建或从其他分组拖入。"
          >
            <template #extra
              ><NSpace justify="center">
                <NButton :disabled="store.managementBusy" @click="createHost"
                  >在此新建主机</NButton
                >
              </NSpace></template
            >
          </NEmpty>
          <div v-else class="host-grid">
            <NCard
              v-for="host in visible"
              :key="host.id"
              size="small"
              hoverable
              tabindex="0"
              :aria-label="
                host.name + '，按回车连接，右键或 Shift+F10 打开操作菜单'
              "
              @pointerdown="groupTree?.startHostDrag($event, host)"
              @dragstart.prevent
              @dblclick="emit('connect', host)"
              @keydown.enter.self.prevent="emit('connect', host)"
              @keydown.self="cardKey($event, host)"
              @contextmenu="openMenu($event, host)"
            >
              <template #header>
                <div class="host-card-title">
                  <NEllipsis class="host-card-name">{{ host.name }}</NEllipsis>
                  <NTag class="host-protocol" type="info" size="small"
                    >SSH</NTag
                  >
                </div>
              </template>
              <template #header-extra>
                <NButton
                  :disabled="store.managementBusy"
                  quaternary
                  circle
                  :type="host.favorite ? 'warning' : 'default'"
                  :aria-label="host.favorite ? '取消收藏' : '收藏'"
                  @dblclick.stop
                  @click.stop="favorite(host)"
                  >{{ host.favorite ? "★" : "☆" }}</NButton
                >
              </template>
              <NSpace vertical>
                <NText depth="3">{{
                  groupPath(store.groups, host.groupId)
                }}</NText>
                <NEllipsis class="selectable-text"
                  >{{ host.username }}@{{
                    host.address.includes(":")
                      ? "[" + host.address + "]"
                      : host.address
                  }}:{{ host.port }}</NEllipsis
                >
                <NSpace v-if="host.tags.length" size="small">
                  <NTag v-for="tag in host.tags" :key="tag" size="small">{{
                    tag
                  }}</NTag>
                </NSpace>
              </NSpace>
            </NCard>
          </div>
        </NScrollbar>
      </section>
    </div>
    <NDropdown
      :show="menu.show"
      :x="menu.x"
      :y="menu.y"
      :options="menuOptions"
      :menu-props="() => ({ 'data-ui-popup-open': menu.show ? '' : undefined })"
      display-directive="show"
      trigger="manual"
      placement="bottom-start"
      @select="selectMenu"
      @clickoutside="menu.show = false"
      @update:show="menu.show = $event"
    />
    <AppDialog
      :open="!!movingHost"
      title="移动主机"
      :locked="store.managementBusy"
      @update:open="
        (value) => {
          if (!value) movingHost = undefined;
        }
      "
    >
      <p>将“{{ movingHost?.name }}”移到：</p>
      <NForm ref="moveForm" :model="{ destination }" :rules="moveRules"
        ><NFormItem label="目标分组" path="destination"
          ><NSelect
            v-model:value="destination"
            :options="moveOptions"
            filterable /></NFormItem
      ></NForm>
      <NText v-if="moveError" type="error" role="alert">{{ moveError }}</NText>
      <template #footer
        ><NSpace justify="end"
          ><NButton
            :disabled="store.managementBusy"
            @click="movingHost = undefined"
            >取消</NButton
          ><NButton
            type="primary"
            :loading="store.managementBusy"
            @click="moveHost"
            >移动</NButton
          ></NSpace
        ></template
      >
    </AppDialog>
  </section>
</template>
<style scoped>
.hosts-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.host-actions {
  flex-shrink: 0;
  margin-bottom: 16px;
}
.hosts-error {
  margin-bottom: 16px;
}
.host-layout {
  display: flex;
  gap: 24px;
  flex: 1;
  min-height: 0;
}
.host-content {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
}
.host-scope {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 16px;
}
.host-scope > :first-child {
  max-width: 100%;
}
.host-list-scroll {
  flex: 1;
  min-height: 0;
}
.host-card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.host-card-name {
  min-width: 0;
}
.host-protocol {
  flex-shrink: 0;
}
.host-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 270px), 1fr));
  gap: 16px;
  padding-bottom: 16px;
}
.hosts-empty {
  margin: 64px 0;
}
@media (max-width: 1000px) {
  .host-layout {
    gap: 16px;
  }
}
</style>

<script setup lang="ts">
import { t } from "../../i18n";
import ScrollFade from "../../components/ui/ScrollFade.vue";
import HostProfileIcon from "./HostProfileIcon.vue";
import ConnectionTypeDropdown from "./ConnectionTypeDropdown.vue";
import { hostEndpoint, hostKindLabel } from "./presentation";
import AssetIcon from "../../components/ui/AssetIcon.vue";
import { NIcon } from "naive-ui";
import { GridOutline, ListOutline, Star, StarOutline } from "@vicons/ionicons5";
import { closeContextMenuOnOutside } from "../../components/ui/contextMenu";
import { confirmAction } from "../../components/ui/confirmation";
import { computed, nextTick, ref, watch, reactive } from "vue";
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
  NList,
  NListItem,
  NRadioButton,
  NRadioGroup,
  NScrollbar,
  NSelect,
  NSpace,
  NTag,
  NText,
} from "naive-ui";
import { validateForm } from "../../services/forms/rules";
import AppDialog from "../../components/ui/AppDialog.vue";
import { errorMessage } from "../../services/desktop/client";
import type { HostKind, HostProfile } from "../../services/desktop/types";
import HostGroupTree from "./HostGroupTree.vue";
import GroupConnectDialog from "./GroupConnectDialog.vue";
import type { TerminalOpenMode } from "../workbench/store";
import HostSearch from "./HostSearch.vue";
import { useSettings } from "../settings/store";
import { descendantIds, groupChoices, groupPath } from "./groups";
import { useHosts } from "./store";
const props = defineProps<{ active: boolean }>();
const emit = defineEmits<{
  connect: [HostProfile];
  connectMany: [HostProfile[], TerminalOpenMode];
  edit: [HostProfile | undefined, boolean?, (string | null)?];
  settings: [];
  local: [];
  create: [HostKind, string | null];
  quick: [HostKind];
}>();
const store = useHosts();
const settings = useSettings();
const search = ref<InstanceType<typeof HostSearch>>();
const groupTree = ref<InstanceType<typeof HostGroupTree>>();
const group = ref("all");
const batchOpen = ref(false);
const batchGroup = ref({ id: "all", hostIds: [] as string[] });
const batchGroupName = computed(() =>
  batchGroup.value.id === "all"
    ? t("hosts.filters.all")
    : groupPath(
        store.groups,
        batchGroup.value.id === "ungrouped" ? null : batchGroup.value.id,
      ),
);
const batchHosts = computed(() => {
  const byId = new Map(store.hosts.map((host) => [host.id, host]));
  return batchGroup.value.hostIds.flatMap((id) => {
    const host = byId.get(id);
    return host ? [host] : [];
  });
});
function connectGroup(id: string) {
  if (!props.active || store.managementBusy || batchGroup.value.hostIds.length)
    return;
  const selected = store.groups.find((item) => item.id === id);
  if (!selected && id !== "all" && id !== "ungrouped") return;
  // 批量范围始终包含后代，不受右侧浏览范围和侧栏主机显示偏好影响。
  const ids = selected ? descendantIds(store.groups, id) : new Set<string>();
  const profiles = store.hosts
    .filter(
      (host) =>
        id === "all" ||
        (id === "ungrouped"
          ? !host.groupId
          : !!host.groupId && ids.has(host.groupId)),
    )
    .sort(
      (a, b) =>
        Number(b.favorite) - Number(a.favorite) || a.name.localeCompare(b.name),
    );
  if (profiles.length === 1) emit("connect", profiles[0]!);
  else if (profiles.length > 1) {
    menu.value.show = false;
    groupTree.value?.cancelDrag();
    batchGroup.value = {
      id,
      hostIds: profiles.map((host) => host.id),
    };
    batchOpen.value = true;
  }
}
function connectMany(profiles: HostProfile[], mode: TerminalOpenMode) {
  if (!props.active) return;
  // 确认期间删除的主机不再连接，也不追加确认范围之外的新主机。
  const byId = new Map(store.hosts.map((host) => [host.id, host]));
  const current = profiles.flatMap((host) => {
    const saved = byId.get(host.id);
    return saved ? [saved] : [];
  });
  if (current.length) emit("connectMany", current, mode);
}
const includeChildren = computed(
  () => settings.value.hostsView.includeChildren,
);
const savingIncludeChildren = ref(false);
const displayMode = computed(() => settings.value.hostsView.displayMode);
const savingDisplayMode = ref(false);
async function setDisplayMode(value: "card" | "list") {
  if (savingDisplayMode.value) return;
  savingDisplayMode.value = true;
  menu.value.show = false;
  groupTree.value?.cancelDrag();
  try {
    await settings.setHostsView({ displayMode: value });
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    savingDisplayMode.value = false;
  }
}
const error = ref("");
const selectedGroup = computed(() =>
  store.groups.find((item) => item.id === group.value),
);
const groupName = computed(() =>
  group.value === "all"
    ? t("hosts.filters.all")
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
function createHost(kind: HostKind = "ssh") {
  emit("create", kind, selectedGroup.value?.id ?? null);
}
const menu = ref({ show: false, hostId: "", x: 0, y: 0 });
const menuOptions = computed(() => [
  { label: t("common.actions.connect"), key: "connect" },
  {
    label: t("common.actions.edit"),
    key: "edit",
    disabled:
      store.managementBusy ||
      store.hosts.find((h) => h.id === menu.value.hostId)?.kind === "workspace",
  },
  {
    label: t("common.actions.duplicate"),
    key: "duplicate",
    disabled:
      store.managementBusy ||
      store.hosts.find((h) => h.id === menu.value.hostId)?.kind === "workspace",
  },
  {
    label: t("hosts.actions.moveToGroup"),
    key: "move",
    disabled: store.managementBusy,
  },
  {
    label: t("common.actions.delete"),
    key: "delete",
    disabled: store.managementBusy,
  },
]);
const movingHost = ref<HostProfile>();
const destination = ref("");
const moveError = ref("");
const moveOptions = computed(() => [
  { value: "", label: t("hosts.groups.ungrouped") },
  ...groupChoices(store.groups),
]);
function openMenu(event: MouseEvent | KeyboardEvent, host: HostProfile) {
  event.preventDefault();
  const target = event.currentTarget as HTMLElement;
  const rect = (
    event instanceof KeyboardEvent
      ? (target.querySelector<HTMLElement>(".n-tree-node--pending") ?? target)
      : target
  ).getBoundingClientRect();
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
    if (key === "edit" && host.kind !== "workspace") emit("edit", host);
    else if (key === "duplicate" && host.kind !== "workspace")
      emit("edit", host, true);
    else if (key === "delete") await remove(host);
    else if (key === "move") {
      movingHost.value = { ...host };
      destination.value = host.groupId ?? "";
      moveError.value = "";
    }
  }
}
const moveModel = reactive({ destination });
const moveForm = ref<FormInst>();
const moveRules: FormRules = {
  destination: {
    required: true,
    validator: () =>
      destination.value === "" ||
      store.groups.some((group) => group.id === destination.value) ||
      new Error(t("hosts.validation.invalidGroup")),
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
        t("hosts.confirmDelete", {
          name: host.name,
        }),
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
      <ConnectionTypeDropdown
        :label="t('hosts.actions.new')"
        primary
        :disabled="store.managementBusy || !active"
        @select="createHost"
      />
      <ConnectionTypeDropdown
        :label="t('hosts.actions.quickConnect')"
        :disabled="!active"
        @select="emit('quick', $event)"
      />
      <NButton @click="emit('local')">
        <template #icon>
          <AssetIcon name="local-terminal" :size="18" />
        </template>
        {{ t("terminal.titles.local") }}
      </NButton>
    </NSpace>
    <NAlert
      v-if="error"
      class="hosts-error"
      type="error"
      closable
      @close="error = ''"
    >
      {{ error }}
    </NAlert>
    <div class="host-layout">
      <HostGroupTree
        ref="groupTree"
        v-model="group"
        :active="active"
        @error="error = $event"
        @connect="emit('connect', $event)"
        @connect-group="connectGroup"
        @edit-host="emit('edit', $event)"
        @host-menu="openMenu"
      />
      <GroupConnectDialog
        v-model:open="batchOpen"
        :active="active"
        :group-name="batchGroupName"
        :hosts="batchHosts"
        @connect="connectMany"
        @after-leave="batchGroup = { id: 'all', hostIds: [] }"
      />
      <section class="host-content" :aria-label="t('hosts.filters.group')">
        <div class="host-scope">
          <NEllipsis>
            <NText strong>{{ groupName }}</NText>
          </NEllipsis>
          <NText depth="3">
            {{ t("hosts.count", { count: visible.length }) }}
          </NText>
          <NCheckbox
            v-if="selectedGroup"
            :checked="includeChildren"
            :disabled="savingIncludeChildren"
            @update:checked="setIncludeChildren"
          >
            {{ t("hosts.filters.includeSubgroups") }}
          </NCheckbox>
          <NRadioGroup
            class="host-display-mode"
            :value="displayMode"
            :disabled="savingDisplayMode"
            name="host-display-mode"
            :aria-label="t('hosts.view.label')"
            @update:value="setDisplayMode"
          >
            <NRadioButton
              value="card"
              :title="t('hosts.view.cards')"
              :aria-label="t('hosts.view.cards')"
            >
              <NIcon :size="18" aria-hidden="true"><GridOutline /></NIcon>
            </NRadioButton>
            <NRadioButton
              value="list"
              :title="t('hosts.view.list')"
              :aria-label="t('hosts.view.list')"
            >
              <NIcon :size="18" aria-hidden="true"><ListOutline /></NIcon>
            </NRadioButton>
          </NRadioGroup>
        </div>
        <ScrollFade class="host-list-scroll" background="var(--bg)" :size="20">
          <NScrollbar>
            <NEmpty
              v-if="!visible.length"
              class="hosts-empty"
              :description="t('hosts.groups.empty')"
            >
              <template #extra>
                <NSpace justify="center">
                  <NButton
                    :disabled="store.managementBusy"
                    @click="createHost()"
                  >
                    {{ t("hosts.actions.createHere") }}
                  </NButton>
                </NSpace>
              </template>
            </NEmpty>
            <div v-else-if="displayMode === 'card'" class="host-grid">
              <NCard
                v-for="host in visible"
                :key="host.id"
                size="small"
                hoverable
                tabindex="0"
                :aria-label="
                  t('hosts.accessibility.connect', {
                    name: host.name,
                  })
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
                    <HostProfileIcon :host="host" />
                    <NEllipsis class="host-card-name">
                      {{ host.name }}
                    </NEllipsis>
                    <NTag class="host-protocol" type="info" size="small">
                      {{ hostKindLabel(host) }}
                    </NTag>
                  </div>
                </template>
                <template #header-extra>
                  <NButton
                    :disabled="store.managementBusy"
                    quaternary
                    circle
                    :type="host.favorite ? 'warning' : 'default'"
                    :aria-label="
                      host.favorite
                        ? t('common.actions.removeFavorite')
                        : t('hosts.actions.favorite')
                    "
                    @dblclick.stop
                    @click.stop="favorite(host)"
                  >
                    <NIcon :size="18" aria-hidden="true">
                      <Star v-if="host.favorite" />
                      <StarOutline v-else />
                    </NIcon>
                  </NButton>
                </template>
                <NSpace vertical>
                  <NText depth="3">
                    {{ groupPath(store.groups, host.groupId) }}
                  </NText>
                  <NEllipsis class="selectable-text">
                    {{ hostEndpoint(host) }}
                  </NEllipsis>
                  <NSpace v-if="host.tags.length" size="small">
                    <NTag v-for="tag in host.tags" :key="tag" size="small">
                      {{ tag }}
                    </NTag>
                  </NSpace>
                </NSpace>
              </NCard>
            </div>
            <NList v-else hoverable bordered size="small" class="host-list">
              <NListItem
                v-for="host in visible"
                :key="host.id"
                tabindex="0"
                :aria-label="
                  t('hosts.accessibility.connect', {
                    name: host.name,
                  })
                "
                @pointerdown="groupTree?.startHostDrag($event, host)"
                @dragstart.prevent
                @dblclick="emit('connect', host)"
                @keydown.enter.self.prevent="emit('connect', host)"
                @keydown.self="cardKey($event, host)"
                @contextmenu="openMenu($event, host)"
              >
                <template #prefix><HostProfileIcon :host="host" /></template>
                <div class="host-list-details">
                  <div class="host-card-title">
                    <NEllipsis class="host-card-name">
                      {{ host.name }}
                    </NEllipsis>
                    <NTag class="host-protocol" type="info" size="small">
                      {{ hostKindLabel(host) }}
                    </NTag>
                  </div>
                  <NEllipsis class="selectable-text">
                    {{ hostEndpoint(host) }}
                  </NEllipsis>
                  <NEllipsis>
                    <NText depth="3">
                      {{ groupPath(store.groups, host.groupId) }}
                    </NText>
                  </NEllipsis>
                  <NSpace
                    v-if="host.tags.length"
                    size="small"
                    class="host-list-tags"
                  >
                    <NTag v-for="tag in host.tags" :key="tag" size="small">
                      {{ tag }}
                    </NTag>
                  </NSpace>
                </div>
                <template #suffix>
                  <NButton
                    :disabled="store.managementBusy"
                    quaternary
                    circle
                    :type="host.favorite ? 'warning' : 'default'"
                    :aria-label="
                      host.favorite
                        ? t('common.actions.removeFavorite')
                        : t('hosts.actions.favorite')
                    "
                    @dblclick.stop
                    @click.stop="favorite(host)"
                  >
                    <NIcon :size="18" aria-hidden="true">
                      <Star v-if="host.favorite" />
                      <StarOutline v-else />
                    </NIcon>
                  </NButton>
                </template>
              </NListItem>
            </NList>
          </NScrollbar>
        </ScrollFade>
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
      @clickoutside="closeContextMenuOnOutside($event, menu)"
      @update:show="menu.show = $event"
    />
    <AppDialog
      :open="!!movingHost"
      :title="t('hosts.actions.move')"
      :locked="store.managementBusy"
      @update:open="
        (value) => {
          if (!value) movingHost = undefined;
        }
      "
    >
      <p>{{ t("hosts.move.prompt", { name: movingHost?.name ?? "" }) }}</p>
      <NForm ref="moveForm" :model="moveModel" :rules="moveRules">
        <NFormItem :label="t('hosts.move.destination')" path="destination">
          <NSelect
            v-model:value="moveModel.destination"
            :options="moveOptions"
            filterable
          />
        </NFormItem>
      </NForm>
      <NText v-if="moveError" type="error" role="alert">{{ moveError }}</NText>
      <template #footer>
        <NSpace justify="end">
          <NButton
            :disabled="store.managementBusy"
            @click="movingHost = undefined"
          >
            {{ t("common.actions.cancel") }}
          </NButton>
          <NButton
            type="primary"
            :loading="store.managementBusy"
            @click="moveHost"
          >
            {{ t("common.actions.move") }}
          </NButton>
        </NSpace>
      </template>
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
.host-display-mode {
  margin-left: auto;
  flex-shrink: 0;
}
/* 图标按钮沿用库默认高度，标签内部用弹性布局消除文字基线偏移。 */
.host-display-mode :deep(.n-radio__label) {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
}
.host-display-mode :deep(.n-icon) {
  display: flex;
}
.host-list {
  margin-bottom: 16px;
}
.host-list :deep(.n-list-item__main) {
  min-width: 0;
}
.host-list-details {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.3fr) minmax(0, 1fr);
  align-items: center;
  gap: 8px 16px;
}
.host-list-tags {
  grid-column: 1 / -1;
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
  .host-list-details {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  }
}
@media (max-width: 760px) {
  .host-list-details {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>

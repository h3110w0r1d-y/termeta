<script setup lang="ts">
import { confirmAction, promptText } from "../../components/ui/confirmation";
import { computed, nextTick, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NCard,
  NDropdown,
  NEllipsis,
  NEmpty,
  NH1,
  NInput,
  NRadioGroup,
  NRadioButton,
  NSpace,
  NTag,
  NText,
  type InputInst,
} from "naive-ui";
import AppSelect from "../../components/ui/AppSelect.vue";
import { desktop, errorMessage } from "../../services/desktop/client";
import type { HostProfile } from "../../services/desktop/types";
import { useHosts } from "./store";
const props = defineProps<{ active: boolean }>();
const search = ref<InputInst>();
watch(
  () => props.active,
  async (active) => {
    if (!active) return;
    // 主机页保活，等待外层显隐更新后再聚焦；不清空已有搜索内容。
    await nextTick();
    if (props.active) search.value?.focus();
  },
  { immediate: true, flush: "post" },
);
const emit = defineEmits<{
  connect: [HostProfile];
  edit: [HostProfile | undefined, boolean?];
  quick: [];
  settings: [];
}>();
const store = useHosts(),
  query = ref(""),
  group = ref("all"),
  filter = ref("all"),
  error = ref("");
const visible = computed(() =>
  store.hosts.filter(
    (h) =>
      (filter.value !== "favorites" || h.favorite) &&
      (group.value === "all" ||
        (group.value === "ungrouped"
          ? !h.groupId
          : h.groupId === group.value)) &&
      [h.name, h.address, h.username, ...h.tags]
        .join(" ")
        .toLowerCase()
        .includes(query.value.toLowerCase()),
  ),
);
const menu = ref({ show: false, hostId: "", x: 0, y: 0 });
const menuOptions = [
  { label: "连接", key: "connect" },
  { label: "编辑", key: "edit" },
  { label: "复制", key: "duplicate" },
  { label: "删除", key: "delete" },
];
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
  else if (key === "edit") emit("edit", host);
  else if (key === "duplicate") emit("edit", host, true);
  else if (key === "delete") await remove(host);
}
watch(
  () => props.active,
  (active) => {
    if (!active) menu.value.show = false;
  },
);
function depth(id: string, seen = new Set<string>()): number {
  if (seen.has(id)) return 0;
  seen.add(id);
  const parent = store.groups.find((g) => g.id === id)?.parentId;
  return parent ? 1 + depth(parent, seen) : 0;
}
const groupOptions = computed(() => [
  { value: "all", label: "所有分组" },
  { value: "ungrouped", label: "未分组" },
  ...store.groups.map((g) => ({
    value: g.id,
    label: "　".repeat(Math.min(8, depth(g.id))) + g.name,
  })),
]);
async function remove(host: HostProfile) {
  if (
    await confirmAction(
      `删除主机“${host.name}”？当前会话不受影响，共享凭据不会删除。`,
    )
  )
    try {
      await store.remove(host);
    } catch (e) {
      error.value = errorMessage(e);
    }
}
async function createGroup() {
  const name = await promptText("分组名称");
  if (!name?.trim()) return;
  try {
    await desktop.saveGroup(
      {
        id: null,
        name: name.trim(),
        parentId: ["all", "ungrouped"].includes(group.value)
          ? null
          : group.value,
        sortOrder: store.groups.length,
      },
      null,
    );
    await store.load();
  } catch (e) {
    error.value = errorMessage(e);
  }
}
async function manageGroup() {
  const g = store.groups.find((g) => g.id === group.value);
  if (!g) return;
  const name = await promptText(
    "修改分组名称（留空表示删除并将子项移至父级）",
    g.name,
  );
  if (name === null) return;
  try {
    if (name.trim())
      await desktop.saveGroup({ ...g, name: name.trim() }, g.revision);
    else if (await confirmAction("确认删除分组并移动子项？")) {
      await desktop.deleteGroup(g.id, g.revision);
      group.value = "all";
    }
    await store.load();
  } catch (e) {
    error.value = errorMessage(e);
  }
}
</script>
<template>
  <section class="hosts-page home-page">
    <header class="page-heading">
      <div>
        <NH1>主机</NH1>
        <NText depth="3">管理你的 SSH 连接，随时回到工作现场。</NText>
      </div>
      <NSpace>
        <NButton @click="emit('quick')">快速连接</NButton>
        <NButton type="primary" @click="emit('edit', undefined)"
          >新建主机</NButton
        >
      </NSpace>
    </header>
    <div class="host-toolbar">
      <NInput
        ref="search"
        v-model:value="query"
        class="host-search"
        :input-props="{ type: 'search', 'aria-label': '搜索主机' }"
        placeholder="搜索主机、地址、用户名或标签…"
      />
      <NRadioGroup v-model:value="filter" aria-label="主机范围">
        <NRadioButton value="all">全部 {{ store.hosts.length }}</NRadioButton>
        <NRadioButton value="favorites">收藏</NRadioButton>
      </NRadioGroup>
    </div>
    <div class="host-group-toolbar">
      <AppSelect
        v-model="group"
        class="group-filter"
        label="筛选分组"
        :options="groupOptions"
      />
      <NButton @click="createGroup">新建分组</NButton>
      <NButton
        :disabled="group === 'all' || group === 'ungrouped'"
        @click="manageGroup"
        >管理分组</NButton
      >
      <NText depth="3" class="result-count">{{ visible.length }} 台主机</NText>
    </div>
    <NAlert v-if="error" type="error" closable @close="error = ''">{{
      error
    }}</NAlert>
    <NEmpty
      v-if="!visible.length"
      class="hosts-empty"
      :description="
        store.hosts.length
          ? '没有符合条件的主机'
          : '添加一台 SSH 主机，或使用快速连接开始。'
      "
    >
      <template #extra>
        <NButton
          v-if="!store.hosts.length"
          type="primary"
          @click="emit('edit', undefined)"
          >新建第一台主机</NButton
        >
        <NButton
          v-else
          @click="
            query = '';
            group = 'all';
            filter = 'all';
          "
          >清除筛选</NButton
        >
      </template>
    </NEmpty>
    <div v-else class="host-grid">
      <NCard
        v-for="host in visible"
        :key="host.id"
        size="small"
        hoverable
        tabindex="0"
        :aria-label="host.name + '，按回车连接，右键或 Shift+F10 打开操作菜单'"
        @dblclick="emit('connect', host)"
        @keydown.enter.self.prevent="emit('connect', host)"
        @keydown.self="cardKey($event, host)"
        @contextmenu="openMenu($event, host)"
      >
        <template #header>
          <div class="host-card-title">
            <NEllipsis class="host-card-name">{{ host.name }}</NEllipsis>
            <NTag class="host-protocol" type="info" size="small">SSH</NTag>
          </div>
        </template>
        <template #header-extra>
          <NButton
            quaternary
            circle
            :type="host.favorite ? 'warning' : 'default'"
            :aria-label="host.favorite ? '取消收藏' : '收藏'"
            @dblclick.stop
            @click.stop="
              store.favorite(host).catch((e) => (error = errorMessage(e)))
            "
            >{{ host.favorite ? "★" : "☆" }}</NButton
          >
        </template>
        <NSpace vertical>
          <NText depth="3">{{
            store.groups.find((g) => g.id === host.groupId)?.name || "未分组"
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
    <NDropdown
      :show="menu.show"
      :x="menu.x"
      :y="menu.y"
      :options="menuOptions"
      :menu-props="() => ({ 'data-ui-popup-open': '' })"
      trigger="manual"
      placement="bottom-start"
      @select="selectMenu"
      @clickoutside="menu.show = false"
      @update:show="menu.show = $event"
    />
  </section>
</template>
<style scoped>
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
.host-toolbar,
.host-group-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 20px;
}
.host-search {
  flex: 1;
  min-width: 180px;
}
.group-filter {
  width: 200px;
}
.result-count {
  margin-left: auto;
}
.host-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 300px), 1fr));
  gap: 16px;
}
.hosts-empty {
  margin: 64px 0;
}
</style>

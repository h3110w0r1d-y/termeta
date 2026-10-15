<script setup lang="ts">
import { confirmAction, promptText } from "../../components/ui/confirmation";
import { computed, ref } from "vue";
import {
  NAlert,
  NButton,
  NCard,
  NEllipsis,
  NEmpty,
  NH1,
  NRadioGroup,
  NRadioButton,
  NSpace,
  NTag,
  NText,
} from "naive-ui";
import AppInput from "../../components/ui/AppInput.vue";
import AppSelect from "../../components/ui/AppSelect.vue";
import { desktop, errorMessage } from "../../services/desktop/client";
import type { HostProfile } from "../../services/desktop/types";
import { useHosts } from "./store";
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
      <AppInput
        v-model="query"
        class="host-search"
        type="search"
        aria-label="搜索主机"
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
        :aria-label="host.name + '，按回车连接'"
        @dblclick="emit('connect', host)"
        @keydown.enter.self.prevent="emit('connect', host)"
      >
        <template #header
          ><NEllipsis>{{ host.name }}</NEllipsis></template
        >
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
          <NSpace size="small">
            <NTag type="info" size="small">SSH</NTag>
            <NTag v-for="tag in host.tags" :key="tag" size="small">{{
              tag
            }}</NTag>
          </NSpace>
        </NSpace>
        <template #action>
          <div @dblclick.stop>
            <NSpace justify="space-between" size="small">
              <NButton
                type="primary"
                secondary
                size="small"
                @click.stop="emit('connect', host)"
                >连接</NButton
              >
              <NSpace size="small">
                <NButton
                  quaternary
                  size="small"
                  @click.stop="emit('edit', host)"
                  >编辑</NButton
                >
                <NButton
                  quaternary
                  size="small"
                  @click.stop="emit('edit', host, true)"
                  >复制</NButton
                >
                <NButton
                  quaternary
                  type="error"
                  size="small"
                  @click.stop="remove(host)"
                  >删除</NButton
                >
              </NSpace>
            </NSpace>
          </div>
        </template>
      </NCard>
    </div>
  </section>
</template>
<style scoped>
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

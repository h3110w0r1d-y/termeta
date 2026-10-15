<script setup lang="ts">
import { confirmAction, promptText } from "../../components/ui/confirmation";
import { computed, ref } from "vue";
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
  selected = ref(""),
  group = ref("all"),
  favoriteOnly = ref(false),
  error = ref("");
const visible = computed(() =>
  store.hosts.filter(
    (h) =>
      (!favoriteOnly.value || h.favorite) &&
      (group.value === "all" || h.groupId === group.value) &&
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
        parentId: group.value === "all" ? null : group.value,
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
  <aside class="sidebar">
    <div class="brand">
      <span class="brand-mark">›_</span>
      <div>Termvexa<small>SSH WORKSPACE</small></div>
    </div>
    <div class="sidebar-section">
      <div class="section-heading">
        连接库 <span>{{ store.hosts.length }}</span>
      </div>
      <button class="primary full" @click="emit('edit', undefined)">
        ＋ 新建主机</button
      ><button class="full subtle" @click="emit('quick')">快速连接</button
      ><input
        v-model="query"
        aria-label="搜索主机"
        placeholder="搜索主机、地址、标签…"
      />
      <div class="filter-row">
        <button
          :class="{ selected: !favoriteOnly }"
          @click="favoriteOnly = false"
        >
          全部主机</button
        ><button
          :class="{ selected: favoriteOnly }"
          @click="favoriteOnly = true"
        >
          ☆ 收藏
        </button>
      </div>
      <div class="group-row">
        <select v-model="group" aria-label="分组">
          <option value="all">所有分组</option>
          <option v-for="g in store.groups" :key="g.id" :value="g.id">
            {{ "　".repeat(Math.min(8, depth(g.id))) }}{{ g.name }}
          </option></select
        ><button class="icon-button" title="新增分组" @click="createGroup">
          ＋</button
        ><button
          class="icon-button"
          :disabled="group === 'all'"
          title="管理分组"
          @click="manageGroup"
        >
          ⋯
        </button>
      </div>
    </div>
    <div class="host-list">
      <p v-if="!visible.length" class="empty-small">
        {{ query ? "没有匹配的主机" : "尚无主机，开始创建连接。" }}
      </p>
      <article
        v-for="host in visible"
        :key="host.id"
        class="host-card"
        :class="{ selected: selected === host.id }"
        tabindex="0"
        @click="selected = host.id"
        @dblclick="emit('connect', host)"
        @keydown.enter.prevent="emit('connect', host)"
      >
        <div class="host-top">
          <span class="host-symbol">▣</span><strong>{{ host.name }}</strong
          ><button
            class="icon-button star"
            :aria-label="host.favorite ? '取消收藏' : '收藏'"
            @click.stop="
              store.favorite(host).catch((e) => (error = errorMessage(e)))
            "
          >
            {{ host.favorite ? "★" : "☆" }}
          </button>
        </div>
        <div class="host-address">
          {{ host.username }}@{{ host.address }}:{{ host.port }}
        </div>
        <div class="tags">
          <span v-for="tag in host.tags" :key="tag">{{ tag }}</span>
        </div>
        <div v-if="selected === host.id" class="host-actions">
          <button @click.stop="emit('connect', host)">连接</button
          ><button @click.stop="emit('edit', host)">编辑</button
          ><button @click.stop="emit('edit', host, true)">复制</button
          ><button class="danger" @click.stop="remove(host)">删除</button>
        </div>
      </article>
    </div>
    <p v-if="error" class="error sidebar-section">{{ error }}</p>
    <footer class="sidebar-footer">
      <span class="status-dot"></span> 本地连接库<button
        class="icon-button"
        aria-label="设置"
        @click="emit('settings')"
      >
        ⚙
      </button>
    </footer>
  </aside>
</template>

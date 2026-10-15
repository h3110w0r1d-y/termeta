<script setup lang="ts">
import { computed, h, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NCheckbox,
  NDataTable,
  NEmpty,
  NFlex,
  NInput,
  NSelect,
  NText,
  type DataTableColumns,
  type DataTableRowKey,
  type DataTableSortState,
} from "naive-ui";
import { desktop, errorMessage } from "../../services/desktop/client";
import type { FileEntry } from "../../services/desktop/types";
import { useHosts } from "../hosts/store";
import type { SftpEndpoint, SftpSelection } from "./workspace";
import FileEntryIcon from "./FileEntryIcon.vue";
import { formatFileSize } from "./format";
import { useSettings } from "../settings/store";

const props = defineProps<{
  endpoint: SftpEndpoint;
  busy: boolean;
  canCopy: boolean;
}>();
const emit = defineEmits<{
  connect: [profileId: string];
  disconnect: [];
  drag: [event: PointerEvent, selection: SftpSelection];
  copy: [selection: SftpSelection];
}>();
const hosts = useHosts();
const profile = ref<string | null>(null);
const options = computed(() =>
  hosts.hosts.map((host) => ({
    label: `${host.name} · ${host.username}@${host.address}:${host.port}`,
    value: host.id,
  })),
);
const directory = ref(".");
const pathInput = ref(".");
const entries = ref<FileEntry[]>([]);
const checked = ref<DataTableRowKey[]>([]);
const loading = ref(false);
const error = ref("");
const hasMore = ref(false);
const settings = useSettings();
const hidden = computed(() => settings.value.sftpShowHidden);
async function updateHidden(show: boolean) {
  try {
    await settings.setSftpShowHidden(show);
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}
const order = ref<DataTableSortState["order"]>("ascend");
const ready = computed(
  () => props.endpoint.state === "connected" && !!props.endpoint.sessionId,
);
const disabled = computed(() => !ready.value || loading.value || props.busy);
const chosen = computed(() =>
  entries.value.filter((entry) => checked.value.includes(entry.name)),
);
const parent: FileEntry = {
  name: "..",
  kind: "directory",
  size: null,
  modified: null,
  permissions: null,
  linkTarget: null,
  targetKind: null,
};
const isDirectory = (entry: FileEntry) =>
  entry.kind === "directory" ||
  (entry.kind === "symlink" && entry.targetKind === "directory");
const rows = computed(() => [
  parent,
  ...entries.value
    .filter((entry) => hidden.value || !entry.name.startsWith("."))
    .sort((a, b) => {
      const group = Number(isDirectory(b)) - Number(isDirectory(a));
      return (
        group ||
        (order.value
          ? (order.value === "descend" ? -1 : 1) * a.name.localeCompare(b.name)
          : 0)
      );
    }),
]);
const label = (entry: FileEntry) =>
  entry.kind === "symlink" && entry.linkTarget !== null
    ? `${entry.name} -> ${entry.linkTarget}`
    : entry.name;
const columns = computed<DataTableColumns<FileEntry>>(() => [
  {
    type: "selection",
    width: 36,
    disabled: (row) => row.name === ".." || disabled.value,
  },
  {
    title: "名称",
    key: "name",
    minWidth: 160,
    sorter: true,
    sortOrder: order.value,
    render: (row) =>
      h("span", { class: "file-name", title: label(row) }, [
        h(FileEntryIcon, { kind: row.kind, targetKind: row.targetKind }),
        h("span", label(row)),
      ]),
  },
  {
    title: "大小",
    key: "size",
    width: 105,
    render: (row) => formatFileSize(row.size),
  },
  {
    title: "权限",
    key: "permissions",
    width: 70,
    render: (row) =>
      row.permissions === null
        ? "—"
        : (row.permissions & 0o7777).toString(8).padStart(3, "0"),
  },
]);
let generation = 0;
async function load(path = directory.value, more = false) {
  const sessionId = props.endpoint.sessionId;
  if (!sessionId || !ready.value || props.busy || loading.value) return;
  const current = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const page = await desktop.sftpList(sessionId, path, more);
    if (generation !== current || props.endpoint.sessionId !== sessionId)
      return;
    directory.value = pathInput.value = page.path;
    entries.value = more ? [...entries.value, ...page.entries] : page.entries;
    if (!more) checked.value = [];
    hasMore.value = page.hasMore;
  } catch (cause) {
    if (generation === current) {
      error.value = errorMessage(cause);
      pathInput.value = directory.value;
    }
  } finally {
    if (generation === current) loading.value = false;
  }
}
watch(
  () => [props.endpoint.sessionId, props.endpoint.state] as const,
  () => {
    generation++;
    loading.value = false;
    entries.value = [];
    checked.value = [];
    hasMore.value = false;
    directory.value = pathInput.value = ".";
    if (ready.value) void load(".");
  },
);
function selection(items = chosen.value): SftpSelection | undefined {
  if (!items.length || disabled.value) return;
  if (
    items.some((entry) => entry.kind !== "file" && entry.kind !== "directory")
  ) {
    error.value = "对拷仅支持普通文件和真实目录，不跟随软链接。";
    return;
  }
  return {
    sessionId: props.endpoint.sessionId!,
    path: directory.value,
    names: items.map((entry) => entry.name),
  };
}
function rowProps(row: FileEntry) {
  return {
    onPointerdown: (event: PointerEvent) => {
      if (
        disabled.value ||
        row.name === ".." ||
        event.button !== 0 ||
        !event.isPrimary ||
        (event.target as HTMLElement).closest(".n-checkbox")
      )
        return;
      if (!checked.value.includes(row.name)) checked.value = [row.name];
      const value = selection();
      if (value) emit("drag", event, value);
    },
    onDblclick: () => {
      if (
        disabled.value ||
        !isDirectory(row) ||
        (row.name === ".." && directory.value === "/")
      )
        return;
      void load(
        row.name === ".."
          ? directory.value.slice(0, directory.value.lastIndexOf("/")) || "/"
          : `${directory.value.replace(/\/$/, "")}/${row.name}`,
      );
    },
  };
}
function copySelected() {
  const value = selection();
  if (value) emit("copy", value);
}
function updateSort(sort: DataTableSortState | DataTableSortState[] | null) {
  order.value = (Array.isArray(sort) ? sort[0] : sort)?.order ?? false;
}
const available = computed(
  () => !disabled.value && directory.value.startsWith("/"),
);
defineExpose({ refresh: () => load(), path: directory, available });
</script>

<template>
  <section class="endpoint-pane">
    <NFlex :wrap="false">
      <NSelect
        v-model:value="profile"
        :options="options"
        filterable
        placeholder="选择已保存的 SSH 主机"
        :disabled="busy || endpoint.opening || !!endpoint.sessionId"
        style="flex: 1; min-width: 0"
      />
      <NButton
        v-if="!endpoint.sessionId"
        :disabled="!profile || busy || endpoint.opening"
        :loading="endpoint.opening"
        @click="profile && emit('connect', profile)"
        >连接</NButton
      >
      <NButton
        v-else
        :disabled="busy || endpoint.opening"
        @click="emit('disconnect')"
        >断开</NButton
      >
    </NFlex>
    <NAlert v-if="endpoint.error || error" type="error" :bordered="false">{{
      endpoint.error || error
    }}</NAlert>
    <template v-if="ready">
      <NFlex :wrap="false">
        <NButton
          :disabled="disabled || directory === '/'"
          @click="load(directory.slice(0, directory.lastIndexOf('/')) || '/')"
          >上级</NButton
        >
        <NButton :disabled="disabled" @click="load('.')">主目录</NButton>
        <NButton :disabled="disabled" @click="load()">刷新</NButton>
      </NFlex>
      <NInput
        v-model:value="pathInput"
        :disabled="disabled"
        placeholder="远端路径"
        @keydown.enter="load(pathInput)"
      />
      <NFlex align="center" justify="space-between"
        ><NCheckbox
          :checked="hidden"
          :disabled="disabled || settings.savingSftpHidden"
          @update:checked="updateHidden"
          >显示隐藏文件</NCheckbox
        ><NButton
          size="small"
          :disabled="disabled || !chosen.length || !canCopy"
          @click="copySelected"
          >复制到另一侧</NButton
        ></NFlex
      >
      <NDataTable
        class="file-table"
        :columns="columns"
        :data="rows"
        :row-key="(row) => row.name"
        :row-props="rowProps"
        :checked-row-keys="checked"
        :loading="loading"
        :virtual-scroll="true"
        :flex-height="true"
        :single-line="false"
        size="small"
        @update:checked-row-keys="(keys) => (checked = keys)"
        @update:sorter="updateSort"
      />
      <NFlex justify="space-between" align="center"
        ><NText depth="3"
          >已加载 {{ entries.length }} 项 · 已选 {{ checked.length }} 项</NText
        ><NButton
          v-if="hasMore"
          size="small"
          :disabled="disabled"
          @click="load(directory, true)"
          >加载更多</NButton
        ></NFlex
      >
    </template>
    <NEmpty
      v-else
      class="empty"
      :description="
        endpoint.sessionId
          ? '正在建立连接或连接已结束'
          : '选择主机以建立独立 SFTP 连接'
      "
    />
  </section>
</template>

<style scoped>
.endpoint-pane {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
  min-height: 0;
  height: 100%;
  padding: 16px;
}
.file-table {
  flex: 1;
  min-height: 140px;
}
.empty {
  margin: auto;
  padding: 24px;
}
:deep(.file-name) {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
:deep(.file-name .n-icon) {
  flex: none;
}
:deep(.file-name span) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>

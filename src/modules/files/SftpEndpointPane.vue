<script setup lang="ts">
import { t } from "@/i18n";
import { validateForm } from "@/services/forms/rules";
import { computed, h, ref, watch, reactive } from "vue";
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
  NForm,
  NFormItem,
  type FormInst,
} from "naive-ui";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { FileEntry } from "@/services/desktop/types";
import { useHosts } from "../hosts/store";
import type { SftpEndpoint, SftpSelection } from "./types";
import { directoryRows, entryLabel, isDirectory } from "./fileEntries";
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
const connectModel = reactive({ profile });
const connectForm = ref<FormInst>();
const connectRules = computed(() => ({
  profile: {
    required: true,
    message: t("hosts.selection.sshRequired"),
    trigger: "change",
  },
}));
async function connect() {
  if (props.busy || props.endpoint.opening || props.endpoint.sessionId) return;
  if ((await validateForm(connectForm.value)) && profile.value)
    emit("connect", profile.value);
}
const options = computed(() =>
  hosts.hosts
    .filter((host) => host.kind === "ssh")
    .map((host) => ({
      label: `${host.name} · ${host.username}@${host.address}:${host.port}`,
      value: host.id,
    })),
);
const directory = ref(".");
const pathInput = ref(".");
const pathModel = reactive({ path: pathInput });
const pathForm = ref<FormInst>();
const pathRules = computed(() => ({
  path: {
    required: true,
    message: t("files.path.placeholder"),
    trigger: ["input", "blur"],
  },
}));
async function navigatePath() {
  if (await validateForm(pathForm.value)) await load(pathInput.value);
}
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
const rows = computed(() =>
  directoryRows(entries.value, hidden.value, order.value),
);
const columns = computed<DataTableColumns<FileEntry>>(() => [
  {
    type: "selection",
    width: 36,
    disabled: (row) => row.name === ".." || disabled.value,
  },
  {
    title: t("common.fields.name"),
    key: "name",
    minWidth: 160,
    sorter: true,
    sortOrder: order.value,
    render: (row) =>
      h("span", { class: "file-name", title: entryLabel(row) }, [
        h(FileEntryIcon, { kind: row.kind, targetKind: row.targetKind }),
        h("span", entryLabel(row)),
      ]),
  },
  {
    title: t("files.fields.size"),
    key: "size",
    width: 105,
    render: (row) => formatFileSize(row.size),
  },
  {
    title: t("files.fields.permissions"),
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
    error.value = t("files.copy.unsupportedType");
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
    <NForm
      ref="connectForm"
      :model="connectModel"
      :rules="connectRules"
      @submit.prevent="connect"
    >
      <NFlex :wrap="false">
        <NFormItem
          :label="t('common.connection.sshHost')"
          path="profile"
          class="path-form"
        >
          <NSelect
            v-model:value="connectModel.profile"
            :options="options"
            filterable
            :placeholder="t('hosts.selection.sshPlaceholder')"
            :disabled="busy || endpoint.opening || !!endpoint.sessionId"
          />
        </NFormItem>
        <NButton
          v-if="!endpoint.sessionId"
          :disabled="busy || endpoint.opening"
          :loading="endpoint.opening"
          @click="connect"
        >
          {{ t("common.actions.connect") }}
        </NButton>
        <NButton
          v-else
          :disabled="busy || endpoint.opening"
          @click="emit('disconnect')"
        >
          {{ t("terminal.actions.disconnect") }}
        </NButton>
      </NFlex>
    </NForm>
    <NAlert v-if="endpoint.error || error" type="error" :bordered="false">
      {{ endpoint.error || error }}
    </NAlert>
    <template v-if="ready">
      <NFlex :wrap="false">
        <NButton
          :disabled="disabled || directory === '/'"
          @click="load(directory.slice(0, directory.lastIndexOf('/')) || '/')"
        >
          {{ t("files.actions.parent") }}
        </NButton>
        <NButton :disabled="disabled" @click="load('.')">
          {{ t("files.actions.home") }}
        </NButton>
        <NButton :disabled="disabled" @click="load()">
          {{ t("common.actions.refresh") }}
        </NButton>
      </NFlex>
      <NForm
        ref="pathForm"
        :model="pathModel"
        :rules="pathRules"
        @submit.prevent="navigatePath"
        class="path-form"
      >
        <NFormItem
          :label="t('files.path.label')"
          path="path"
          :show-label="false"
        >
          <NInput
            v-model:value="pathModel.path"
            :disabled="disabled"
            :placeholder="t('files.path.label')"
          />
        </NFormItem>
      </NForm>
      <NFlex align="center" justify="space-between">
        <NCheckbox
          :checked="hidden"
          :disabled="disabled || settings.savingSftpHidden"
          @update:checked="updateHidden"
        >
          {{ t("files.actions.showHidden") }}
        </NCheckbox>
        <NButton
          size="small"
          :disabled="disabled || !chosen.length || !canCopy"
          @click="copySelected"
        >
          {{ t("files.actions.copyToOtherPane") }}
        </NButton>
      </NFlex>
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
      <NFlex justify="space-between" align="center">
        <NText depth="3">
          {{
            t("files.selection.loaded", {
              loaded: entries.length,
              selected: checked.length,
            })
          }}
        </NText>
        <NButton
          v-if="hasMore"
          size="small"
          :disabled="disabled"
          @click="load(directory, true)"
        >
          {{ t("common.actions.loadMore") }}
        </NButton>
      </NFlex>
    </template>
    <NEmpty
      v-else
      class="empty"
      :description="
        endpoint.sessionId
          ? t('terminal.state.unavailable')
          : t('files.hostPlaceholder')
      "
    />
  </section>
</template>

<style scoped>
.path-form {
  flex: 1;
  min-width: 0;
}
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

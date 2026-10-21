<script setup lang="ts">
import { t } from "@/i18n";
import { computed, ref, watch, onBeforeUnmount } from "vue";
import { NAlert, NButton, NText } from "naive-ui";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { FileEntry } from "@/services/desktop/types";
import { useHosts } from "../hosts/store";
import HostProfileIcon from "../hosts/HostProfileIcon.vue";
import { hostEndpoint } from "../hosts/presentation";
import { isSessionTerminal } from "../sessions/control";
import type { SftpEndpoint, SftpSelection } from "./types";
import { isDirectory } from "./fileEntries";
import SftpToolbar from "./SftpToolbar.vue";
import SftpFileTable from "./SftpFileTable.vue";
import SftpFileActions from "./SftpFileActions.vue";
import SftpHostPicker from "./SftpHostPicker.vue";
import { IconArrowsExchange } from "@tabler/icons-vue";
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
const connectedHost = computed(() =>
  hosts.hosts.find((host) => host.id === props.endpoint.profileId),
);
const connecting = computed(
  () =>
    props.endpoint.opening ||
    (!!props.endpoint.sessionId &&
      !ready.value &&
      !isSessionTerminal(props.endpoint.state)),
);
const directory = ref(".");
const entries = ref<FileEntry[]>([]);
const checked = ref<string[]>([]);
const loading = ref(false);
const error = ref("");
const hasMore = ref(false);
const listSession = ref<string>();
const settings = useSettings();
const hidden = computed(() => settings.sftpShowHidden);
async function updateHidden(show: boolean) {
  try {
    await settings.setSftpShowHidden(show);
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}
const mutationBusy = ref(false);
const fileActions = ref<InstanceType<typeof SftpFileActions>>();
const ready = computed(
  () => props.endpoint.state === "connected" && !!props.endpoint.sessionId,
);
const disabled = computed(
  () => !ready.value || loading.value || props.busy || mutationBusy.value,
);
const stale = computed(() => listSession.value !== props.endpoint.sessionId);
const chosen = computed(() => {
  const names = new Set(checked.value);
  return entries.value.filter((entry) => names.has(entry.name));
});
let generation = 0;
let disposed = false;
async function load(path = directory.value, more = false) {
  const sessionId = props.endpoint.sessionId;
  if (!sessionId || disabled.value || disposed) return;
  const current = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const page = await desktop.sftpList(sessionId, path, more);
    if (
      disposed ||
      generation !== current ||
      props.endpoint.sessionId !== sessionId
    )
      return;
    listSession.value = sessionId;
    directory.value = page.path;
    entries.value = more ? [...entries.value, ...page.entries] : page.entries;
    if (!more) checked.value = [];
    hasMore.value = page.hasMore;
  } catch (cause) {
    if (generation === current) {
      error.value = errorMessage(cause);
      listSession.value = undefined;
    }
  } finally {
    if (generation === current) loading.value = false;
  }
}
watch(
  () => [props.endpoint.sessionId, props.endpoint.state] as const,
  () => {
    generation++;
    listSession.value = undefined;
    loading.value = false;
    entries.value = [];
    checked.value = [];
    hasMore.value = false;
    directory.value = ".";
    if (ready.value) void load(".");
  },
  { immediate: true },
);
function selection(items = chosen.value): SftpSelection | undefined {
  if (!items.length || disabled.value || stale.value) return;
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
function navigateEntry(row: FileEntry) {
  if (!disabled.value && !stale.value && isDirectory(row))
    void load(`${directory.value.replace(/\/$/, "")}/${row.name}`);
}
function startDrag(event: PointerEvent, items: FileEntry[]) {
  const value = selection(items);
  if (value) emit("drag", event, value);
}
function copySelected() {
  const value = selection();
  if (value) emit("copy", value);
}
const available = computed(
  () => !disabled.value && !stale.value && directory.value.startsWith("/"),
);
defineExpose({ refresh: () => load(), path: directory, available });
onBeforeUnmount(() => {
  disposed = true;
  generation++;
});
</script>

<template>
  <section class="endpoint-pane">
    <div v-if="ready" class="connection-header">
      <HostProfileIcon v-if="connectedHost" :host="connectedHost" :size="32" />
      <div class="connection-info">
        <NText class="connection-name" :title="connectedHost?.name">
          {{ connectedHost?.name ?? t("common.connection.sshHost") }}
        </NText>
        <NText
          v-if="connectedHost"
          depth="3"
          class="connection-address"
          :title="hostEndpoint(connectedHost)"
        >
          {{ hostEndpoint(connectedHost) }}
        </NText>
      </div>
      <NButton
        size="small"
        :disabled="busy || endpoint.opening || mutationBusy || loading"
        :loading="endpoint.opening"
        @click="emit('disconnect')"
      >
        {{ t("terminal.actions.disconnect") }}
      </NButton>
    </div>
    <NAlert v-if="endpoint.error || error" type="error" :bordered="false">
      {{ endpoint.error || error }}
    </NAlert>
    <template v-if="ready">
      <SftpToolbar
        :path="directory"
        :disabled="disabled"
        :show-hidden="hidden"
        :saving-hidden="settings.savingSftpHidden"
        @navigate="load"
        @refresh="load()"
        @hidden="updateHidden"
      >
        <NButton
          size="small"
          :disabled="disabled || stale || !chosen.length || !canCopy"
          @click="copySelected"
        >
          <template #icon>
            <IconArrowsExchange :size="16" aria-hidden="true" />
          </template>
          {{ t("files.actions.copyToOtherPane") }}
        </NButton>
      </SftpToolbar>
      <SftpFileTable
        v-model:selected="checked"
        :entries="entries"
        :show-hidden="hidden"
        :disabled="disabled || stale"
        :loading="loading"
        :at-root="directory === '/'"
        @navigate="navigateEntry"
        @drag="startDrag"
        @contextmenu="(event, row) => fileActions?.open(event, row)"
      />
      <div class="pane-status">
        <NText depth="3">
          {{
            t("files.selection.loaded", {
              loaded: entries.filter(
                (entry) => hidden || !entry.name.startsWith("."),
              ).length,
              selected: checked.length,
            })
          }}
        </NText>
        <NButton
          v-if="hasMore"
          size="tiny"
          :disabled="disabled || entries.length >= 10000"
          @click="load(directory, true)"
        >
          {{ t("common.actions.loadMore") }}
        </NButton>
      </div>
      <NText v-if="entries.length >= 10000 && hasMore" type="warning">
        {{ t("files.errors.tooManyEntries") }}
      </NText>
      <SftpFileActions
        ref="fileActions"
        :session-id="endpoint.sessionId"
        :path="directory"
        :entries="chosen"
        :disabled="!ready || loading || busy || stale"
        @busy="mutationBusy = $event"
        @changed="load()"
      />
    </template>
    <SftpHostPicker
      v-else
      :disabled="busy"
      :connecting="connecting"
      :profile-id="endpoint.profileId"
      @connect="emit('connect', $event)"
    />
  </section>
</template>

<style scoped>
.connection-header {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border);
}
.connection-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}
.connection-name,
.connection-address {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.connection-address {
  font-size: 12px;
}
.endpoint-pane {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
  min-height: 0;
  height: 100%;
  padding: 12px;
}
.pane-status {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  min-height: 26px;
  border-top: 1px solid var(--border);
  padding-top: 8px;
  font-size: 12px;
}
</style>

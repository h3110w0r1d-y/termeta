<script setup lang="ts">
import { computed, ref } from "vue";
import { NButton, NEmpty, NInput, NScrollbar, NSpin, NText } from "naive-ui";
import { IconArrowRight, IconSearch } from "@tabler/icons-vue";
import { t } from "@/i18n";
import { useHosts } from "../hosts/store";
import { useHostSearch } from "../hosts/useHostSearch";
import { hostEndpoint } from "../hosts/presentation";
import { groupPath } from "../hosts/groups";
import HostProfileIcon from "../hosts/HostProfileIcon.vue";

const props = defineProps<{
  disabled: boolean;
  connecting: boolean;
  profileId: string | null;
}>();
const emit = defineEmits<{ connect: [profileId: string] }>();
const hosts = useHosts();
const query = ref("");
const matches = useHostSearch(query, true);
const sshHosts = computed(() =>
  matches.value.filter((host) => host.kind === "ssh"),
);
function connect(profileId: string) {
  if (!props.disabled && !props.connecting) emit("connect", profileId);
}
</script>

<template>
  <div class="host-picker" :aria-busy="connecting">
    <NInput
      v-model:value="query"
      clearable
      :placeholder="t('files.hosts.search')"
      :aria-label="t('files.hosts.search')"
    >
      <template #prefix><IconSearch :size="17" aria-hidden="true" /></template>
    </NInput>
    <NText depth="3" class="picker-hint">
      {{ t("files.hostPlaceholder") }}
    </NText>
    <NScrollbar class="host-scroll">
      <ul
        v-if="sshHosts.length"
        class="host-list"
        :aria-label="t('common.connection.sshHost')"
      >
        <li v-for="host in sshHosts" :key="host.id">
          <NButton
            quaternary
            block
            class="host-row"
            :disabled="disabled || connecting"
            :title="`${host.name} · ${hostEndpoint(host)} · ${groupPath(hosts.groups, host.groupId)}`"
            @click="connect(host.id)"
          >
            <HostProfileIcon :host="host" :size="34" />
            <span class="host-info">
              <span class="host-name">{{ host.name }}</span>
              <NText depth="3" class="host-detail">
                {{ hostEndpoint(host) }}
              </NText>
              <NText v-if="host.groupId" depth="3" class="host-detail">
                {{ groupPath(hosts.groups, host.groupId) }}
              </NText>
            </span>
            <NSpin v-if="connecting && profileId === host.id" :size="16" />
            <IconArrowRight
              v-else
              :size="17"
              class="connect-icon"
              aria-hidden="true"
            />
          </NButton>
        </li>
      </ul>
      <NEmpty
        v-else
        class="host-empty"
        :description="
          query.trim() ? t('hosts.search.noResults') : t('files.hosts.empty')
        "
      />
    </NScrollbar>
    <NText v-if="connecting" depth="3" class="picker-hint" role="status">
      {{ t("terminal.state.connecting") }}
    </NText>
  </div>
</template>

<style scoped>
.host-picker {
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
  min-height: 0;
  min-width: 0;
}
.picker-hint {
  font-size: 12px;
}
.host-scroll {
  flex: 1;
  min-height: 0;
}
.host-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.host-row {
  height: auto;
  padding: 12px;
}
.host-row :deep(.n-button__content) {
  width: 100%;
  min-width: 0;
  gap: 12px;
  text-align: left;
}
.host-info {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  gap: 4px;
}
.host-name,
.host-detail {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.4;
}
.host-name {
  font-weight: 500;
}
.host-detail {
  font-size: 12px;
}
.connect-icon {
  flex-shrink: 0;
  opacity: 0.6;
}
.host-empty {
  padding: 48px 16px;
}
</style>

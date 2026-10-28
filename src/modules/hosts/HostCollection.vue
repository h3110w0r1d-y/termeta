<script setup lang="ts">
import {
  NButton,
  NCard,
  NEllipsis,
  NEmpty,
  NList,
  NListItem,
  NScrollbar,
  NSpace,
  NTag,
  NText,
} from "naive-ui";
import { t } from "@/i18n";
import ScrollFade from "@/components/ui/ScrollFade.vue";
import HostProfileIcon from "./HostProfileIcon.vue";
import type { HostGroup, HostProfile } from "./model";
import { groupPath } from "./groups";
import { hostEndpoint, hostKindLabel } from "./presentation";

// 选择范围、连接与拖动由页面协调；这里统一卡片/列表的展示和 DOM 标记。
defineProps<{
  hosts: HostProfile[];
  groups: HostGroup[];
  mode: "card" | "list";
  selectedIds: string[];
  focusedId: string | undefined;
  busy: boolean;
}>();
const emit = defineEmits<{
  create: [];
  select: [event: MouseEvent, host: HostProfile];
  connect: [event: MouseEvent, host: HostProfile];
  drag: [event: PointerEvent, host: HostProfile];
  keydown: [event: KeyboardEvent, host: HostProfile];
  menu: [event: MouseEvent, host: HostProfile];
}>();
</script>

<template>
  <ScrollFade
    class="host-list-scroll"
    background="var(--bg)"
    :size="20"
    data-host-drop-area
  >
    <NScrollbar>
      <NEmpty
        v-if="!hosts.length"
        class="hosts-empty"
        :description="t('hosts.groups.empty')"
      >
        <template #extra>
          <NSpace justify="center">
            <NButton :disabled="busy" @click="emit('create')">
              {{ t("hosts.actions.createHere") }}
            </NButton>
          </NSpace>
        </template>
      </NEmpty>
      <div
        v-else-if="mode === 'card'"
        class="host-grid"
        role="listbox"
        aria-multiselectable="true"
        :aria-label="t('hosts.filters.group')"
      >
        <NCard
          v-for="host in hosts"
          :key="host.id"
          size="small"
          hoverable
          class="host-item"
          :class="{ 'host-selected': selectedIds.includes(host.id) }"
          :tabindex="
            focusedId === host.id || (!focusedId && host.id === hosts[0]?.id)
              ? 0
              : -1
          "
          role="option"
          :aria-selected="selectedIds.includes(host.id)"
          :data-host-order-id="host.id"
          :aria-label="
            t('hosts.accessibility.connect', {
              name: host.name,
            })
          "
          @pointerdown="emit('drag', $event, host)"
          @dragstart.prevent
          @click="emit('select', $event, host)"
          @dblclick="emit('connect', $event, host)"
          @keydown.self="emit('keydown', $event, host)"
          @contextmenu="emit('menu', $event, host)"
        >
          <template #header>
            <div class="host-card-title host-card-heading">
              <div class="host-card-icon">
                <HostProfileIcon :host="host" />
              </div>
              <div class="host-card-name-row">
                <NEllipsis class="host-card-name">
                  {{ host.name }}
                </NEllipsis>
                <NTag class="host-protocol" type="info" size="small">
                  {{ hostKindLabel(host) }}
                </NTag>
              </div>
              <div class="host-card-group">
                <NEllipsis>
                  <NText depth="3">
                    {{ groupPath(groups, host.groupId) }}
                  </NText>
                </NEllipsis>
              </div>
            </div>
          </template>
          <NSpace vertical>
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
      <NList
        v-else
        hoverable
        bordered
        size="small"
        class="host-list"
        role="listbox"
        aria-multiselectable="true"
        :aria-label="t('hosts.filters.group')"
      >
        <NListItem
          v-for="host in hosts"
          :key="host.id"
          class="host-item"
          :class="{ 'host-selected': selectedIds.includes(host.id) }"
          :tabindex="
            focusedId === host.id || (!focusedId && host.id === hosts[0]?.id)
              ? 0
              : -1
          "
          role="option"
          :aria-selected="selectedIds.includes(host.id)"
          :data-host-order-id="host.id"
          :aria-label="
            t('hosts.accessibility.connect', {
              name: host.name,
            })
          "
          @pointerdown="emit('drag', $event, host)"
          @dragstart.prevent
          @click="emit('select', $event, host)"
          @dblclick="emit('connect', $event, host)"
          @keydown.self="emit('keydown', $event, host)"
          @contextmenu="emit('menu', $event, host)"
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
                {{ groupPath(groups, host.groupId) }}
              </NText>
            </NEllipsis>
            <NSpace v-if="host.tags.length" size="small" class="host-list-tags">
              <NTag v-for="tag in host.tags" :key="tag" size="small">
                {{ tag }}
              </NTag>
            </NSpace>
          </div>
        </NListItem>
      </NList>
    </NScrollbar>
  </ScrollFade>
</template>

<style scoped>
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
/* 选中与悬浮分开；只开放地址正文的原生选字。 */
.host-item {
  position: relative;
  -webkit-user-select: none;
  user-select: none;
}
.host-item.host-selected,
.host-item:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.host-item.host-selected {
  background: color-mix(in srgb, var(--accent) 12%, var(--surface));
}
.host-item[data-host-drop-position]::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  height: 3px;
  background: var(--accent);
  pointer-events: none;
  z-index: 1;
}
.host-item[data-host-drop-position="before"]::after {
  top: 0;
}
.host-item[data-host-drop-position="after"]::after {
  bottom: 0;
}
.host-grid .host-item[data-host-drop-position]::after {
  width: 3px;
  height: auto;
  top: 0;
  bottom: 0;
  right: auto;
}
.host-grid .host-item[data-host-drop-position="after"]::after {
  left: auto;
  right: 0;
}
.host-card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.host-card-heading {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0 8px;
}
.host-card-icon {
  grid-row: span 2;
  display: flex;
  align-items: center;
}
.host-card-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.host-card-group {
  min-width: 0;
  font-size: 13px;
  font-weight: normal;
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

<script setup lang="ts">
import { Star } from "@vicons/ionicons5";
import { computed, h, ref, watch, type HTMLAttributes } from "vue";
import {
  NIcon,
  NSelect,
  NEllipsis,
  NSpace,
  NText,
  type SelectInst,
  type SelectOption,
} from "naive-ui";
import SearchPreferences from "../settings/SearchPreferences.vue";
import type { HostProfile } from "../../services/desktop/types";
import { useHosts } from "./store";
import { hostEndpoint, hostKindLabel } from "./presentation";
import { groupPath } from "./groups";
import { useHostSearch } from "./useHostSearch";

const props = defineProps<{ active: boolean }>();
const emit = defineEmits<{ connect: [HostProfile] }>();
const store = useHosts();
const shown = ref(false);
const query = ref("");
const menuVisible = computed(
  () => props.active && shown.value && !!query.value.trim(),
);
const menuProps = computed<HTMLAttributes & Record<string, string | undefined>>(
  () => ({
    "data-ui-popup-open": menuVisible.value ? "" : undefined,
    "data-application-shortcuts": "",
  }),
);
const input = ref<SelectInst>();
const matches = useHostSearch(query);
const options = computed<SelectOption[]>(() => {
  const result: SelectOption[] = matches.value
    .slice(0, 100)
    .map((host) => ({ value: host.id, label: host.name }));
  if (matches.value.length > 100)
    result.push({
      value: "more",
      label: `共 ${matches.value.length} 台，仅显示前100台，请缩小搜索范围`,
      disabled: true,
    });
  return result;
});
function renderLabel(option: SelectOption) {
  const host = store.hosts.find((host) => host.id === option.value);
  if (!host) return String(option.label);
  return h(NSpace, { vertical: true, size: 0 }, () => [
    h(NEllipsis, null, () => [
      host.favorite
        ? h(NIcon, { size: 16, role: "img", "aria-label": "已收藏" }, () =>
            h(Star),
          )
        : null,
      host.name,
    ]),
    h(
      NText,
      { depth: 3 },
      () =>
        `${hostKindLabel(host)} · ${hostEndpoint(host)} · ${groupPath(store.groups, host.groupId)}`,
    ),
  ]);
}
function select(id: string | number) {
  const host = store.hosts.find((host) => host.id === id);
  if (host) {
    shown.value = false;
    query.value = "";
    emit("connect", host);
  }
}
watch(shown, (value) => {
  if (!value) query.value = "";
});
watch(
  () => props.active,
  (active) => {
    if (!active) {
      shown.value = false;
      input.value?.blurInput();
    }
  },
);
// 库的focus只高亮选择器外壳；保险箱需要真实输入焦点。
defineExpose({ focus: () => input.value?.focusInput() });
</script>
<template>
  <div class="host-search">
    <NSelect
      ref="input"
      :value="null"
      :show="menuVisible"
      display-directive="show"
      :options="options"
      :render-label="renderLabel"
      :menu-props="menuProps"
      filterable
      remote
      clear-filter-after-select
      placeholder="搜索全部主机…"
      aria-label="搜索全部主机"
      @search="query = $event"
      @update:show="shown = $event"
      @update:value="select"
    >
      <template #header>
        <!-- 鼠标修改字段时保留输入焦点与光标；键盘Tab仍可进入复选框。 -->
        <div
          @mousedown.prevent
          @click="input?.focusInput()"
          @keydown.stop
          @keydown.esc="shown = false"
        >
          <SearchPreferences compact />
        </div>
      </template>
      <template #empty>
        {{ query.trim() ? "没有匹配的主机" : "输入关键词搜索全部主机" }}
      </template>
    </NSelect>
  </div>
</template>
<style scoped>
.host-search {
  flex-shrink: 0;
  margin-bottom: 20px;
}
</style>

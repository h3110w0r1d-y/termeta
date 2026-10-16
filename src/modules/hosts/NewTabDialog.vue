<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import {
  NButton,
  NEmpty,
  NFlex,
  NInput,
  NList,
  NListItem,
  NScrollbar,
  NText,
  type InputInst,
} from "naive-ui";
import AppDialog from "../../components/ui/AppDialog.vue";
import SearchPreferences from "../settings/SearchPreferences.vue";
import type { HostProfile } from "../../services/desktop/types";
import { useHosts } from "./store";
import { groupPath } from "./groups";
import { useHostSearch } from "./useHostSearch";
const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{
  "update:open": [boolean];
  connect: [HostProfile];
  quick: [];
  "after-leave": [];
}>();
const hosts = useHosts();
const query = ref("");
const input = ref<InputInst>();
const results = useHostSearch(query, true);
const visible = computed(() => results.value.slice(0, 100));
const selected = ref(0);
const list = ref<HTMLElement>();
let pending: HostProfile | "quick" | undefined;
watch(
  () => props.open,
  (open) => {
    if (open) {
      query.value = "";
      selected.value = 0;
      pending = undefined;
    }
  },
);
watch(visible, () => {
  selected.value = 0;
});
function focus() {
  if (props.open) input.value?.focus();
}
function choose(host: HostProfile) {
  if (!props.open || pending) return;
  pending = host;
  emit("update:open", false);
}
function quick() {
  if (!props.open || pending) return;
  pending = "quick";
  emit("update:open", false);
}
function afterLeave() {
  // 等旧弹窗与焦点陷阱完全退场，再创建终端或显示快速连接，避免两层弹窗争抢焦点。
  const action = pending;
  pending = undefined;
  if (action === "quick") emit("quick");
  else if (action) emit("connect", action);
  emit("after-leave");
}
async function move(offset: number) {
  if (!visible.value.length) return;
  selected.value =
    (selected.value + offset + visible.value.length) % visible.value.length;
  await nextTick();
  list.value
    ?.querySelector<HTMLElement>(`[data-search-index="${selected.value}"]`)
    ?.scrollIntoView({ block: "nearest" });
}
function arrow(event: KeyboardEvent, offset: number) {
  if (event.isComposing) return;
  event.preventDefault();
  void move(offset);
}
function enter(event: KeyboardEvent) {
  if (event.isComposing) return;
  event.preventDefault();
  const host = visible.value[selected.value];
  if (host) choose(host);
}
</script>
<template>
  <AppDialog
    :open="open"
    title="新建标签页"
    description="选择已保存的主机，或使用快速连接。"
    @update:open="emit('update:open', $event)"
    @after-enter="focus"
    @after-leave="afterLeave"
  >
    <NFlex vertical :size="16">
      <NInput
        ref="input"
        v-model:value="query"
        autofocus
        clearable
        placeholder="搜索全部主机…"
        aria-label="搜索新标签页主机"
        :input-props="{
          maxlength: 256,
          role: 'combobox',
          'aria-autocomplete': 'list',
          'aria-controls': 'new-tab-results',
          'aria-expanded': !!visible.length,
          'aria-activedescendant': visible[selected]
            ? `new-tab-result-${selected}`
            : undefined,
        }"
        @keydown.down="arrow($event, 1)"
        @keydown.up="arrow($event, -1)"
        @keydown.enter="enter"
      />
      <div @mousedown.prevent @click="focus"><SearchPreferences compact /></div>
      <NScrollbar style="max-height: 40dvh">
        <div ref="list">
          <NList
            v-if="visible.length"
            :show-divider="false"
            class="host-results"
            id="new-tab-results"
            role="listbox"
            aria-label="可连接的主机"
          >
            <NListItem v-for="(host, index) in visible" :key="host.id">
              <NButton
                block
                :type="selected === index ? 'primary' : 'default'"
                :secondary="selected === index"
                :quaternary="selected !== index"
                class="host-result"
                :data-search-index="index"
                :id="`new-tab-result-${index}`"
                role="option"
                :aria-selected="selected === index"
                @click="choose(host)"
              >
                <NFlex vertical align="start" :size="4" class="result-text">
                  <NText>{{ host.favorite ? "★ " : "" }}{{ host.name }}</NText>
                  <NText depth="3">
                    {{ host.username }}@{{
                      host.address.includes(":")
                        ? `[${host.address}]`
                        : host.address
                    }}:{{ host.port }} ·
                    {{ groupPath(hosts.groups, host.groupId) }}
                  </NText>
                </NFlex>
              </NButton>
            </NListItem>
          </NList>
          <NEmpty
            v-else
            :description="
              query.trim() ? '没有匹配的主机' : '还没有保存主机，可使用快速连接'
            "
          />
        </div>
      </NScrollbar>
      <NText v-if="results.length > 100" depth="3">
        共 {{ results.length }} 台，仅显示前100台，请缩小搜索范围。
      </NText>
    </NFlex>
    <template #footer>
      <NFlex justify="space-between">
        <NButton @click="quick">快速连接</NButton>
        <NButton @click="emit('update:open', false)">取消</NButton>
      </NFlex>
    </template>
  </AppDialog>
</template>
<style scoped>
.host-results :deep(.n-list-item) {
  padding-block: 0;
}
.host-results :deep(.n-list-item + .n-list-item) {
  margin-top: 4px;
}
.host-result {
  height: auto;
  padding: 12px;
}
.host-result :deep(.n-button__content) {
  width: 100%;
  min-width: 0;
}
.result-text {
  min-width: 0;
  width: 100%;
  white-space: normal;
  overflow-wrap: anywhere;
  text-align: left;
}
</style>

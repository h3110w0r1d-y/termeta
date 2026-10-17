<script setup lang="ts">
import { t } from "../../i18n";
import { watch } from "vue";
import { NButton, NSpace, NText } from "naive-ui";
import AppDialog from "../../components/ui/AppDialog.vue";
import type { HostProfile } from "../../services/desktop/types";
import type { TerminalOpenMode } from "../workbench/store";

const props = defineProps<{
  open: boolean;
  active: boolean;
  groupName: string;
  hosts: HostProfile[];
}>();
const emit = defineEmits<{
  "update:open": [boolean];
  connect: [HostProfile[], TerminalOpenMode];
  "after-leave": [];
}>();
let pending: { hosts: HostProfile[]; mode: TerminalOpenMode } | undefined;
watch(
  () => props.open,
  (open) => {
    if (open) pending = undefined;
  },
);
watch(
  () => props.active,
  (active) => {
    if (!active) cancel();
  },
);
function cancel() {
  pending = undefined;
  emit("update:open", false);
}
function choose(mode: TerminalOpenMode) {
  if (!props.open || !props.active || pending || !props.hosts.length) return;
  pending = { hosts: [...props.hosts], mode };
  emit("update:open", false);
}
function afterLeave() {
  // 完全退场后才创建会话，避免批量信任/认证提示与确认窗争抢焦点。
  const action = pending;
  pending = undefined;
  if (action && props.active) emit("connect", action.hosts, action.mode);
  emit("after-leave");
}
</script>

<template>
  <AppDialog
    :open="open"
    :title="t('批量连接')"
    @update:open="cancel"
    @after-leave="afterLeave"
  >
    <p>
      {{
        t("“{groupName}”中共有 {count} 台主机（包含子分组），是否批量连接？", {
          groupName: groupName,
          count: hosts.length,
        })
      }}
    </p>
    <NText depth="3">{{ t("选择在独立标签或分屏工作区中打开。") }}</NText>
    <NText v-if="hosts.some((host) => host.kind === 'workspace')" depth="3">
      {{ t("已保存的工作区将按原布局打开。") }}
    </NText>
    <template #footer>
      <NSpace justify="end">
        <NButton @click="cancel">{{ t("取消") }}</NButton>
        <NButton :disabled="!hosts.length" @click="choose('tabs')">
          {{ t("在独立标签中打开") }}
        </NButton>
        <NButton
          type="primary"
          :disabled="!hosts.length"
          @click="choose('workspace')"
        >
          {{ t("在工作区中打开") }}
        </NButton>
      </NSpace>
    </template>
  </AppDialog>
</template>

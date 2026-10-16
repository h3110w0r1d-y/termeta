<script setup lang="ts">
import { watch } from "vue";
import { NButton, NSpace, NText } from "naive-ui";
import AppDialog from "../../components/ui/AppDialog.vue";
import type { HostProfile } from "../../services/desktop/types";
import type { TerminalOpenMode } from "../workspace/store";

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
    title="批量连接"
    @update:open="cancel"
    @after-leave="afterLeave"
  >
    <p>
      “{{ groupName }}”中共有
      {{ hosts.length }} 台主机（包含子分组），是否批量连接？
    </p>
    <NText depth="3">
      请选择终端的打开方式。工作区将使用分割视图展示这些终端。
    </NText>
    <NText v-if="hosts.some((host) => host.kind === 'workspace')" depth="3">
      已保存的工作区会各自按原布局打开；所选打开方式仅应用于普通连接。
    </NText>
    <template #footer>
      <NSpace justify="end">
        <NButton @click="cancel">取消</NButton>
        <NButton :disabled="!hosts.length" @click="choose('tabs')">
          在独立标签中打开
        </NButton>
        <NButton
          type="primary"
          :disabled="!hosts.length"
          @click="choose('workspace')"
        >
          在工作区中打开
        </NButton>
      </NSpace>
    </template>
  </AppDialog>
</template>

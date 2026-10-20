<script setup lang="ts">
import { t } from "@/i18n";
import { computed, h, onBeforeUnmount, onMounted, watch } from "vue";
import { NButton, NDropdown, NIcon, type DropdownOption } from "naive-ui";
import {
  IconChevronDown,
  IconCpu,
  IconTerminal2,
  IconWorld,
} from "@tabler/icons-vue";
import type { HostKind } from "@/services/desktop/types";

const props = withDefaults(
  defineProps<{ label: string; primary?: boolean; disabled?: boolean }>(),
  {
    primary: false,
    disabled: false,
  },
);
const emit = defineEmits<{ select: [HostKind] }>();
const shown = defineModel<boolean>("show", { default: false });
const icon = (component: typeof IconTerminal2) => () =>
  h(NIcon, { size: 18, "aria-hidden": true }, () => h(component));
const options = computed<DropdownOption[]>(() => [
  { key: "ssh", label: "SSH", icon: icon(IconTerminal2) },
  {
    key: "telnet",
    label: "Telnet",
    icon: icon(IconWorld),
  },
  {
    key: "serial",
    label: t("common.connection.serial"),
    icon: icon(IconCpu),
  },
]);
watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) shown.value = false;
  },
);
function close() {
  shown.value = false;
}
function navigationKey(event: KeyboardEvent) {
  if (!shown.value) return;
  if (event.key === "Tab") close();
  else if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    close();
  }
}
onMounted(() => {
  // 悬浮菜单也支持键盘打开，离开时释放库的菜单键盘处理。
  window.addEventListener("keydown", navigationKey, true);
  window.addEventListener("blur", close);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", navigationKey, true);
  window.removeEventListener("blur", close);
});
function select(key: string) {
  shown.value = false;
  if (key === "ssh" || key === "serial" || key === "telnet")
    emit("select", key);
}
</script>
<template>
  <NDropdown
    v-model:show="shown"
    trigger="hover"
    placement="bottom-start"
    :options="options"
    :disabled="disabled"
    :menu-props="() => ({ 'data-ui-popup-open': shown ? '' : undefined })"
    @select="select"
    @clickoutside="close"
  >
    <NButton
      :type="primary ? 'primary' : 'default'"
      :disabled="disabled"
      aria-haspopup="menu"
      :aria-expanded="shown"
      @click="shown = true"
      @keydown.down.prevent="shown = true"
      @keydown.esc.stop.prevent="close"
    >
      {{ label }}
      <template #icon>
        <NIcon :size="16" aria-hidden="true"><IconChevronDown /></NIcon>
      </template>
    </NButton>
  </NDropdown>
</template>

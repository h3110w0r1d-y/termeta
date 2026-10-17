<script setup lang="ts">
import { t } from "../../i18n";
import { AddOutline, RefreshOutline, CloseOutline } from "@vicons/ionicons5";
import { NIcon, NTabs, NTab } from "naive-ui";
import AppCheckbox from "../../components/ui/AppCheckbox.vue";
import AppSelect from "../../components/ui/AppSelect.vue";
import AppInput from "../../components/ui/AppInput.vue";
import AppButton from "../../components/ui/AppButton.vue";
import { computed, ref } from "vue";
import {
  commandDefinitions,
  defaults,
  keyLabel,
  parseKeymap,
  conflict,
  isMac,
} from "../../services/commands/registry";
import { downloadJson } from "./themes";
import { parseJson } from "../../services/data/json";
import type { Binding, Modifier } from "./model";
import { clone } from "../../services/data/clone";
import { confirmAction } from "../../components/ui/confirmation";
import { errorMessage } from "../../services/desktop/client";
import type { AppSettings } from "./model";
const keyFile = ref<HTMLInputElement>();
const draft = defineModel<AppSettings>("settings", { required: true });
const newId = () => crypto.randomUUID();
const error = ref(""),
  recording = ref(""),
  shortcutSearch = ref(""),
  category = ref("application");
const bindings = computed(() =>
  draft.value.keymap.bindings.filter(
    (b) =>
      b.scope === category.value &&
      (
        t(
          commandDefinitions.find((d) => d[0] === b.commandId)?.[1] ??
            b.commandId,
        ) +
        " " +
        keyLabel(b)
      )
        .toLowerCase()
        .includes(shortcutSearch.value.toLowerCase()),
  ),
);
function record(event: KeyboardEvent, binding: Binding) {
  event.preventDefault();
  event.stopPropagation();
  if (event.isComposing) return;
  if (event.key === "Escape") {
    recording.value = "";
    return;
  }
  if (["Control", "Meta", "Alt", "Shift"].includes(event.key)) return;
  const modifiers: Modifier[] = [];
  if (event.ctrlKey) modifiers.push("Control");
  if (event.metaKey) modifiers.push("Meta");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  binding.trigger = { type: "key", key: event.key, modifiers };
  binding.enabled = true;
  recording.value = "";
}
function resetBinding(binding: Binding) {
  const original = defaults().find((d) => d.commandId === binding.commandId)!;
  Object.assign(binding, clone(original), { id: binding.id });
}
function removeBinding(binding: Binding) {
  draft.value.keymap.bindings = draft.value.keymap.bindings.filter(
    (b) => b.id !== binding.id,
  );
}
async function exportKeys() {
  try {
    await downloadJson("termeta-keymap.json", draft.value.keymap);
  } catch (e) {
    error.value = errorMessage(e);
  }
}
async function importKeys(event: Event) {
  try {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    if (file.size > 1048576) throw new Error(t("文件过大"));
    const keymap = parseKeymap(parseJson(await file.text()));
    if (
      await confirmAction(
        t("导入 {count} 个绑定并替换当前编辑内容？", {
          count: keymap.bindings.length,
        }),
      )
    )
      draft.value.keymap = keymap;
  } catch (e) {
    error.value = errorMessage(e);
  }
}
</script>
<template>
  <div>
    <div class="button-row">
      <NTabs
        v-model:value="category"
        type="segment"
        :aria-label="t('快捷键分类')"
      >
        <NTab name="application">{{ t("全局快捷键") }}</NTab>
        <NTab name="terminal">{{ t("终端快捷键") }}</NTab>
      </NTabs>
      <AppInput v-model="shortcutSearch" :placeholder="t('搜索动作或按键…')" />
    </div>
    <div v-for="b in bindings" :key="b.id" class="shortcut-row">
      <AppCheckbox v-model="b.enabled">
        {{
          t(
            commandDefinitions.find((d) => d[0] === b.commandId)?.[1] ??
              b.commandId,
          )
        }}
      </AppCheckbox>
      <template v-if="b.trigger.type === 'pointer'">
        <AppSelect
          v-model="b.trigger.modifiers[0]"
          :options="[
            { value: 'Primary', label: t('Cmd / Ctrl + 点击') },
            { value: 'Alt', label: t('Alt + 点击') },
            { value: 'Shift', label: t('Shift + 点击') },
          ]"
        />
      </template>
      <AppButton
        v-else
        :class="{ recording: recording === b.id }"
        @click="recording = b.id"
        data-shortcut-recorder
        @keydown="record($event, b)"
      >
        {{
          recording === b.id
            ? t("按组合键，Esc 取消")
            : keyLabel(b) || t("未绑定")
        }}
      </AppButton>
      <AppButton
        class="icon-button"
        :title="t('增加同动作绑定')"
        @click="
          draft.keymap.bindings.push({
            ...clone(b),
            id: newId(),
            enabled: false,
          })
        "
      >
        <NIcon :size="16" aria-hidden="true"><AddOutline /></NIcon>
      </AppButton>
      <AppButton
        class="icon-button"
        :title="t('恢复此动作默认')"
        @click="resetBinding(b)"
      >
        <NIcon :size="16" aria-hidden="true"><RefreshOutline /></NIcon>
      </AppButton>
      <AppButton
        class="icon-button"
        :title="t('删除绑定')"
        @click="removeBinding(b)"
      >
        <NIcon :size="16" aria-hidden="true"><CloseOutline /></NIcon>
      </AppButton>
    </div>
    <p v-if="conflict(draft.keymap.bindings)" class="error">
      {{ conflict(draft.keymap.bindings) }}
    </p>
    <p class="muted">
      {{ t("将 Ctrl+C 等组合键设为快捷键后，它们将不再发送到远端。") }}
    </p>
    <p v-if="isMac" class="muted">
      {{
        t(
          "Cmd+, 固定打开系统菜单中的设置，此组合不受此处绑定影响；可为打开设置录制其他组合键。",
        )
      }}
    </p>
    <div class="button-row">
      <AppButton @click="draft.keymap.bindings = defaults()">
        {{ t("全部恢复默认") }}
      </AppButton>
      <AppButton @click="exportKeys">{{ t("导出配置") }}</AppButton>
      <AppButton @click="keyFile?.click()">{{ t("导入配置") }}</AppButton>
      <input
        ref="keyFile"
        hidden
        type="file"
        accept=".json,application/json"
        @change="importKeys"
      />
    </div>
  </div>
  <p v-if="error" class="error" role="alert">{{ error }}</p>
</template>

<style scoped>
.shortcut-row {
  display: flex;
  gap: 10px;
  align-items: center;
  padding: 9px 0;
  border-bottom: 1px solid var(--border);
}
.shortcut-row .ui-label {
  flex: 1;
}
.shortcut-row .ui-select-trigger {
  width: 180px;
}
.shortcut-row > .n-button:not(.icon-button) {
  min-width: 170px;
}
.recording {
  outline: 2px solid var(--accent);
}
</style>

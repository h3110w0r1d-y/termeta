<script setup lang="ts">
import { t } from "../../i18n";
import { IconPlus, IconRefresh, IconX } from "@tabler/icons-vue";
import { NIcon, NTabs, NTab } from "naive-ui";
import AppCheckbox from "../../components/ui/AppCheckbox.vue";
import AppSelect from "../../components/ui/AppSelect.vue";
import AppInput from "../../components/ui/AppInput.vue";
import AppButton from "../../components/ui/AppButton.vue";
import { computed, ref } from "vue";
import {
  commandLabel,
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
      (commandLabel(b.commandId) + " " + keyLabel(b))
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
    if (file.size > 1048576)
      throw new Error(t("shortcuts.validation.fileTooLarge"));
    const keymap = parseKeymap(parseJson(await file.text()));
    if (
      await confirmAction(
        t("shortcuts.import.confirm", {
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
        :aria-label="t('shortcuts.categories.label')"
      >
        <NTab name="application">
          {{ t("shortcuts.categories.application") }}
        </NTab>
        <NTab name="terminal">{{ t("shortcuts.categories.terminal") }}</NTab>
      </NTabs>
      <AppInput
        v-model="shortcutSearch"
        :placeholder="t('shortcuts.search.placeholder')"
      />
    </div>
    <div v-for="b in bindings" :key="b.id" class="shortcut-row">
      <AppCheckbox v-model="b.enabled">
        {{ commandLabel(b.commandId) }}
      </AppCheckbox>
      <template v-if="b.trigger.type === 'pointer'">
        <AppSelect
          v-model="b.trigger.modifiers[0]"
          :options="[
            { value: 'Primary', label: t('shortcuts.triggers.primaryClick') },
            { value: 'Alt', label: t('shortcuts.triggers.altClick') },
            { value: 'Shift', label: t('shortcuts.triggers.shiftClick') },
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
            ? t("shortcuts.recording.hint")
            : keyLabel(b) || t("shortcuts.recording.unbound")
        }}
      </AppButton>
      <AppButton
        class="icon-button"
        :title="t('shortcuts.actions.addBinding')"
        @click="
          draft.keymap.bindings.push({
            ...clone(b),
            id: newId(),
            enabled: false,
          })
        "
      >
        <NIcon :size="16" aria-hidden="true"><IconPlus /></NIcon>
      </AppButton>
      <AppButton
        class="icon-button"
        :title="t('shortcuts.actions.resetCommand')"
        @click="resetBinding(b)"
      >
        <NIcon :size="16" aria-hidden="true"><IconRefresh /></NIcon>
      </AppButton>
      <AppButton
        class="icon-button"
        :title="t('shortcuts.actions.deleteBinding')"
        @click="removeBinding(b)"
      >
        <NIcon :size="16" aria-hidden="true"><IconX /></NIcon>
      </AppButton>
    </div>
    <p v-if="conflict(draft.keymap.bindings)" class="error">
      {{ conflict(draft.keymap.bindings) }}
    </p>
    <p class="muted">
      {{ t("shortcuts.warning.remoteKeys") }}
    </p>
    <p v-if="isMac" class="muted">
      {{ t("shortcuts.hint.nativeSettings") }}
    </p>
    <div class="button-row">
      <AppButton @click="draft.keymap.bindings = defaults()">
        {{ t("shortcuts.actions.resetAll") }}
      </AppButton>
      <AppButton @click="exportKeys">
        {{ t("shortcuts.actions.export") }}
      </AppButton>
      <AppButton @click="keyFile?.click()">
        {{ t("shortcuts.actions.import") }}
      </AppButton>
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

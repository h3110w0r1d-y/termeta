<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  NAlert,
  NConfigProvider,
  NEmpty,
  NForm,
  NFormItem,
  NList,
  NListItem,
  NSpin,
  NTabPane,
  NTabs,
  type FormInst,
  type FormRules,
} from "naive-ui";
import AppButton from "@/components/ui/AppButton.vue";
import AppInput from "@/components/ui/AppInput.vue";
import AppNumberField from "@/components/ui/AppNumberField.vue";
import AppSelect from "@/components/ui/AppSelect.vue";
import { t } from "@/i18n";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { Snippet } from "@/services/desktop/types";
import { requiredText } from "@/services/forms/rules";
import { terminalFontOptions } from "../settings/fonts";
import { useSettings } from "../settings/store";
import { useLiveSettings } from "../settings/useLiveSettings";
import { builtinSchemes, resolveScheme } from "../settings/themes";
import SchemeThumbnail from "../settings/SchemeThumbnail.vue";
import { schemeForTerminal, terminalRegistry } from "../terminal/registry";
import { useTerminalSurface } from "../terminal/useTerminalSurface";
import { useBackupRestored } from "../backup/useBackupRestored";
import { useWorkbenchStore } from "./store";

const workbench = useWorkbenchStore();
const settings = useSettings();
const {
  theme,
  themeOverrides,
  style: surfaceStyle,
} = useTerminalSurface(
  () =>
    (workbench.activeTerminal
      ? schemeForTerminal(workbench.activeTerminal)
      : resolveScheme(
          settings.value.defaultSchemeId,
          settings.value.customSchemes,
        )
    ).colors,
);
const form = ref<FormInst>();
const {
  draft,
  error: settingsError,
  busy: savingSettings,
  retry,
} = useLiveSettings(form);
const page = ref("themes");
const error = ref("");
const busy = ref(false);
const loading = ref(false);
const snippets = ref<Snippet[]>([]);
const query = ref("");
const visible = computed(
  () =>
    workbench.sidebarOpen &&
    !!(workbench.activeGroup || workbench.activeLogTab),
);
const schemes = computed(() => [
  ...builtinSchemes,
  ...settings.value.customSchemes,
]);
const selectedScheme = computed(
  () =>
    workbench.activeTerminal?.schemeId ??
    (workbench.activeTerminal?.target.source === "saved"
      ? settings.value.hostSchemes[workbench.activeTerminal.target.profileId]
      : undefined) ??
    settings.value.defaultSchemeId,
);
const filtered = computed(() => {
  const search = query.value.trim().toLowerCase();
  return snippets.value.filter((item) =>
    `${item.name}\n${item.content}`.toLowerCase().includes(search),
  );
});
const canSend = computed(() => {
  const terminal = workbench.activeTerminal;
  return (
    terminal?.state === "connected" &&
    !!terminal.sessionId &&
    !workbench.closingTerminalIds.includes(terminal.terminalId) &&
    !workbench.drag
  );
});
const rules = computed<FormRules>(() => ({
  fontFamily: requiredText("settings.terminal.font"),
  fontSize: {
    required: true,
    type: "number",
    min: 10,
    max: 32,
    message: t("settings.validation.fontSize"),
    trigger: ["change", "blur"],
  },
}));
let loadRevision = 0;
async function load() {
  const revision = ++loadRevision;
  loading.value = true;
  error.value = "";
  try {
    const items = await desktop.listSnippets();
    if (revision === loadRevision) snippets.value = items;
  } catch (cause) {
    if (revision === loadRevision) error.value = errorMessage(cause);
  } finally {
    if (revision === loadRevision) loading.value = false;
  }
}
watch(
  () => visible.value && page.value === "snippets",
  (active) => {
    if (active) void load();
  },
);
useBackupRestored(
  async () => {
    if (visible.value && page.value === "snippets") await load();
  },
  (cause) => {
    error.value = errorMessage(cause);
  },
);
async function selectScheme(id: string) {
  if (busy.value) return;
  const terminal = workbench.activeTerminal;
  const group = workbench.activeGroup;
  const profileId =
    terminal?.target.source === "saved" ? terminal.target.profileId : undefined;
  busy.value = true;
  error.value = "";
  try {
    // 主机继承和全局默认同次提交，避免其他窗口的主机配色更新被旧映射覆盖。
    await settings.patch({
      defaultSchemeId: id,
      ...(profileId ? { hostSchemes: { [profileId]: null } } : {}),
    });
    if (
      terminal &&
      group &&
      workbench.groupForTerminal(terminal.terminalId) === group
    )
      workbench.setTerminalScheme(terminal.terminalId);
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}
async function send(item: Snippet, execute: boolean) {
  const terminal = workbench.activeTerminal;
  if (!terminal || !canSend.value || busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    await terminalRegistry.pasteText(terminal.terminalId, item.content, {
      expectedSessionId: terminal.sessionId,
      execute,
    });
    if (workbench.activeTerminal === terminal)
      terminalRegistry.focus(terminal.terminalId);
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <NConfigProvider abstract :theme="theme" :theme-overrides="themeOverrides">
    <aside
      id="workbench-sidebar"
      v-show="visible"
      class="workbench-sidebar"
      :style="surfaceStyle"
      :aria-label="t('workbench.sidebar.label')"
    >
      <NTabs
        v-model:value="page"
        type="line"
        justify-content="space-evenly"
        class="sidebar-tabs"
        :animated="false"
        :pane-style="{
          flex: 1,
          minHeight: 0,
          overflow: 'auto',
          overscrollBehavior: 'contain',
        }"
      >
        <NTabPane
          name="themes"
          :tab="t('workbench.sidebar.themes')"
          display-directive="show"
        >
          <NForm
            ref="form"
            :model="draft"
            :rules="rules"
            :show-require-mark="false"
            :show-feedback="false"
            label-placement="top"
            class="sidebar-form"
            @submit.prevent
          >
            <NFormItem :label="t('settings.terminal.font')" path="fontFamily">
              <AppSelect
                v-model="draft.fontFamily"
                :options="terminalFontOptions"
              />
            </NFormItem>
            <NFormItem :label="t('settings.terminal.fontSize')" path="fontSize">
              <AppNumberField v-model="draft.fontSize" :min="10" :max="32" />
            </NFormItem>
          </NForm>
          <NAlert v-if="settingsError" type="error">
            {{ settingsError }}
            <AppButton :disabled="savingSettings" @click="retry">
              {{ t("common.actions.retry") }}
            </AppButton>
          </NAlert>
          <p class="muted">{{ t("workbench.sidebar.themeHint") }}</p>
          <div class="scheme-list">
            <AppButton
              v-for="scheme in schemes"
              :key="scheme.id"
              class="scheme-option"
              :variant="selectedScheme === scheme.id ? 'primary' : 'default'"
              :aria-pressed="selectedScheme === scheme.id"
              :disabled="busy"
              @click="selectScheme(scheme.id)"
            >
              <span class="scheme-content">
                <SchemeThumbnail :scheme="scheme" />
                <span>{{ scheme.name }}</span>
              </span>
            </AppButton>
          </div>
        </NTabPane>
        <NTabPane
          name="snippets"
          :tab="t('common.navigation.snippets')"
          display-directive="show"
        >
          <AppInput
            v-model="query"
            type="search"
            :aria-label="t('snippets.search.label')"
            :placeholder="t('snippets.search.placeholder')"
          />
          <p v-if="!canSend" class="muted">
            {{ t("workbench.sidebar.noTerminal") }}
          </p>
          <NSpin
            v-if="loading"
            size="small"
            :description="t('snippets.loading')"
          />
          <NEmpty
            v-else-if="!filtered.length"
            :description="
              snippets.length
                ? t('snippets.search.noResults')
                : t('snippets.empty')
            "
          />
          <NList v-else>
            <NListItem v-for="item in filtered" :key="item.id">
              <div class="snippet-item">
                <strong>{{ item.name }}</strong>
                <pre>{{ item.content }}</pre>
                <div class="snippet-actions">
                  <AppButton
                    :disabled="busy || !canSend"
                    @click="send(item, false)"
                  >
                    {{ t("terminal.actions.paste") }}
                  </AppButton>
                  <AppButton
                    :disabled="busy || !canSend"
                    @click="send(item, true)"
                  >
                    {{ t("workbench.sidebar.execute") }}
                  </AppButton>
                </div>
              </div>
            </NListItem>
          </NList>
        </NTabPane>
      </NTabs>
      <NAlert v-if="error" type="error" closable @close="error = ''">
        {{ error }}
      </NAlert>
    </aside>
  </NConfigProvider>
</template>
<style scoped>
.workbench-sidebar {
  flex: 0 0 300px;
  width: 300px;
  min-width: 0;
  min-height: 0;
  padding: 0 12px 12px;
  border-left: 1px solid var(--border);
  background: var(--surface);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.sidebar-tabs {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
/* 未启用动画时面板直接参与 flex 布局，导航固定、各页面独立滚动。 */
.sidebar-tabs :deep(.n-tabs-nav) {
  flex: none;
}
.sidebar-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.scheme-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
.scheme-option {
  width: 100%;
  height: auto;
  padding: 8px;
}
.scheme-option :deep(.n-button__content) {
  width: 100%;
  min-width: 0;
}
.scheme-content :deep(.scheme-thumbnail) {
  flex: none;
  align-self: center;
}
.scheme-content {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  width: 100%;
}
.scheme-content > span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.snippet-item {
  min-width: 0;
  width: 100%;
  overflow-wrap: anywhere;
}
.snippet-item pre {
  margin: 8px 0;
  max-height: 7.5em;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font: 12px/1.5 monospace;
}
.snippet-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>

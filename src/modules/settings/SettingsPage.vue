<script setup lang="ts">
import { t } from "@/i18n";
import {
  NColorPicker,
  NButton,
  NMenu,
  NForm,
  NFormItem,
  type FormInst,
  type FormItemInst,
  type FormRules,
  NRadioGroup,
  NRadioButton,
  type SelectRenderLabel,
} from "naive-ui";
import AppNumberField from "@/components/ui/AppNumberField.vue";
import AppCheckbox from "@/components/ui/AppCheckbox.vue";
import AppSelect from "@/components/ui/AppSelect.vue";
import SystemDetectionSettings from "./SystemDetectionSettings.vue";
import SearchPreferences from "./SearchPreferences.vue";
import { requiredText, integerRange } from "@/services/forms/rules";
import { computed, h, reactive, ref, type Component } from "vue";
import {
  IconCloudUpload,
  IconKeyboard,
  IconPalette,
  IconSettings,
  IconShieldLock,
} from "@tabler/icons-vue";
import AppScrollArea from "@/components/ui/AppScrollArea.vue";
import ThemeSettings from "./ThemeSettings.vue";
import ShortcutSettings from "./ShortcutSettings.vue";
import VaultSecuritySettings from "./VaultSecuritySettings.vue";
import BackupPanel from "../backup/BackupPanel.vue";
import { useLiveSettings } from "./useLiveSettings";
import { terminalFontFamily, terminalFontOptions } from "./fonts";
// NSelect 同时用此渲染器显示菜单与当前值；字体声明按可见选项自动加载本地资源。
const renderFontLabel: SelectRenderLabel = (option) => {
  const name = typeof option.label === "string" ? option.label : "";
  return h("span", { style: { fontFamily: terminalFontFamily(name) } }, name);
};
const form = ref<FormInst>();
const { draft, flush: flushSettings } = useLiveSettings(form);
const security = ref<InstanceType<typeof VaultSecuritySettings>>();
const backup = ref<InstanceType<typeof BackupPanel>>();
async function flush() {
  if (security.value?.busy || backup.value?.busy) return false;
  return flushSettings();
}
defineExpose({ flush });
type SettingsSection =
  "general" | "security" | "backup" | "themes" | "shortcuts";
const section = ref<SettingsSection>("general");
// 首次进入时挂载，之后仅切换显隐，保留编辑、滚动和进行中的操作。
const visitedSections = reactive(new Set<SettingsSection>(["general"]));
const navigationOptions = computed(() => {
  const items: { key: SettingsSection; label: string; icon: Component }[] = [
    {
      key: "general",
      label: t("settings.sections.general"),
      icon: IconSettings,
    },
    {
      key: "security",
      label: t("settings.sections.security"),
      icon: IconShieldLock,
    },
    {
      key: "backup",
      label: t("settings.sections.backup"),
      icon: IconCloudUpload,
    },
    { key: "themes", label: t("terminal.form.theme"), icon: IconPalette },
    {
      key: "shortcuts",
      label: t("settings.sections.shortcuts"),
      icon: IconKeyboard,
    },
  ];
  return items.map((item) => ({
    key: item.key,
    label: item.label,
    icon: () => h(item.icon, { size: 18, "aria-hidden": true }),
  }));
});
function selectSection(key: string) {
  const item = navigationOptions.value.find((option) => option.key === key);
  if (!item) return;
  section.value = item.key;
  visitedSections.add(item.key);
}
const primaryColorItem = ref<FormItemInst>();
const mouseActionOptions = computed(() => [
  { value: "none", label: t("settings.mouseAction.none") },
  { value: "paste", label: t("terminal.actions.paste") },
]);
const hostSortOptions = computed(() => [
  { value: "recent", label: t("settings.hostSort.recent") },
  { value: "manual", label: t("settings.hostSort.manual") },
]);
function resetPrimaryColor() {
  draft.value.primaryColor = "";
  // 按钮直接更新共享字段后，使用表单项公开接口重新校验。
  void primaryColorItem.value
    ?.validate({ trigger: "change" })
    .catch(() => undefined);
}
const rules = computed<FormRules>(() => ({
  hostSortMode: requiredText("settings.general.hostSortMode"),
  sftpPosition: requiredText("settings.general.sftpPosition"),
  tabPosition: requiredText("settings.general.tabPosition"),
  tabDoubleClickAction: requiredText("settings.general.tabDoubleClickAction"),
  primaryColor: {
    trigger: ["change", "blur"],
    validator: (_, value: string) =>
      value === "" ||
      /^#[0-9a-f]{6}$/i.test(value) ||
      new Error(t("settings.validation.accentColor")),
  },
  fontFamily: requiredText("settings.terminal.font"),
  fontSize: {
    required: true,
    type: "number",
    min: 10,
    max: 32,
    message: t("settings.validation.fontSize"),
    trigger: ["blur", "change"],
  },
  scrollback: integerRange("settings.terminal.scrollback", 1000, 50000),
  terminalRightClickAction: requiredText("settings.terminal.rightClickAction"),
  terminalMiddleClickAction: requiredText(
    "settings.terminal.middleClickAction",
  ),
}));
</script>

<template>
  <section class="settings-page" :aria-label="t('settings.title')">
    <nav class="settings-navigation" :aria-label="t('settings.title')">
      <NMenu
        :root-indent="18"
        :value="section"
        :options="navigationOptions"
        @update:value="selectSection"
      />
    </nav>
    <div class="settings-content">
      <div v-show="section === 'general'" class="settings-pane">
        <AppScrollArea
          class="settings-scroll"
          :label="t('settings.sections.general')"
        >
          <div class="settings-section">
            <h1>{{ t("settings.sections.general") }}</h1>
            <NForm
              ref="form"
              :model="draft"
              :rules="rules"
              label-placement="top"
              @submit.prevent
              class="form-grid"
            >
              <NFormItem
                :label="t('settings.general.language')"
                path="language"
              >
                <AppSelect
                  v-model="draft.language"
                  :options="[
                    {
                      value: 'system',
                      label: t('settings.language.system'),
                    },
                    { value: 'en-US', label: 'English' },
                    { value: 'zh-CN', label: t('settings.languages.zhCN') },
                    { value: 'zh-TW', label: t('settings.languages.zhTW') },
                    { value: 'ja-JP', label: t('settings.languages.jaJP') },
                    { value: 'ko-KR', label: t('settings.languages.koKR') },
                  ]"
                />
              </NFormItem>
              <NFormItem
                :label="t('settings.general.tabPosition')"
                path="tabPosition"
              >
                <NRadioGroup
                  v-model:value="draft.tabPosition"
                  :aria-label="t('settings.general.tabPosition')"
                >
                  <NRadioButton value="top">
                    {{ t("settings.tabPosition.top") }}
                  </NRadioButton>
                  <NRadioButton value="left">
                    {{ t("settings.tabPosition.left") }}
                  </NRadioButton>
                </NRadioGroup>
              </NFormItem>
              <NFormItem
                :label="t('settings.general.sftpPosition')"
                path="sftpPosition"
              >
                <NRadioGroup
                  v-model:value="draft.sftpPosition"
                  :aria-label="t('settings.general.sftpPosition')"
                >
                  <NRadioButton value="left">
                    {{ t("settings.sftpPosition.left") }}
                  </NRadioButton>
                  <NRadioButton value="right">
                    {{ t("settings.sftpPosition.right") }}
                  </NRadioButton>
                </NRadioGroup>
              </NFormItem>
              <NFormItem
                :label="t('settings.general.hostSortMode')"
                path="hostSortMode"
              >
                <div class="host-sort-field">
                  <AppSelect
                    v-model="draft.hostSortMode"
                    :options="hostSortOptions"
                    aria-describedby="settings-host-sort-description"
                  />
                  <p id="settings-host-sort-description" class="muted">
                    {{ t("settings.hostSort.description") }}
                  </p>
                </div>
              </NFormItem>
              <NFormItem
                :label="t('settings.general.tabDoubleClick')"
                path="tabDoubleClickAction"
              >
                <AppSelect
                  v-model="draft.tabDoubleClickAction"
                  :options="[
                    { value: 'none', label: t('settings.doubleClick.none') },
                    {
                      value: 'close',
                      label: t('settings.doubleClick.close'),
                    },
                    {
                      value: 'duplicate',
                      label: t('workbench.actions.duplicateTab'),
                    },
                    { value: 'split', label: t('workbench.actions.split') },
                  ]"
                />
              </NFormItem>
              <NFormItem
                ref="primaryColorItem"
                :label="t('settings.general.accentColor')"
                path="primaryColor"
              >
                <div class="primary-color-row">
                  <NColorPicker
                    :value="draft.primaryColor || null"
                    :modes="['hex']"
                    :show-alpha="false"
                    :aria-label="t('settings.general.accentColor')"
                    @update:value="draft.primaryColor = $event ?? ''"
                  />
                  <NButton @click="resetPrimaryColor">
                    {{ t("common.actions.reset") }}
                  </NButton>
                </div>
              </NFormItem>
              <NFormItem :label="t('settings.terminal.font')" path="fontFamily">
                <AppSelect
                  v-model="draft.fontFamily"
                  :options="terminalFontOptions"
                  :render-label="renderFontLabel"
                />
              </NFormItem>
              <NFormItem
                :label="t('settings.terminal.fontSize')"
                path="fontSize"
              >
                <AppNumberField v-model="draft.fontSize" :min="10" :max="32" />
              </NFormItem>
              <NFormItem
                :label="t('settings.terminal.scrollback')"
                path="scrollback"
              >
                <AppNumberField
                  v-model="draft.scrollback"
                  :min="1000"
                  :max="50000"
                />
              </NFormItem>
              <NFormItem
                :label="t('settings.terminal.rightClickAction')"
                path="terminalRightClickAction"
              >
                <AppSelect
                  v-model="draft.terminalRightClickAction"
                  :options="mouseActionOptions"
                />
              </NFormItem>
              <NFormItem
                :label="t('settings.terminal.middleClickAction')"
                path="terminalMiddleClickAction"
              >
                <AppSelect
                  v-model="draft.terminalMiddleClickAction"
                  :options="mouseActionOptions"
                />
              </NFormItem>
              <div class="punctuation-field span-2">
                <AppCheckbox
                  v-model="draft.convertChinesePunctuation"
                  aria-describedby="settings-punctuation-description"
                >
                  {{ t("settings.terminal.convertChinesePunctuation") }}
                </AppCheckbox>
                <p id="settings-punctuation-description" class="muted">
                  {{ t("settings.terminal.convertChinesePunctuationHint") }}
                </p>
              </div>
              <AppCheckbox v-model="draft.confirmClose" class="span-2">
                {{ t("settings.security.confirmClose") }}
              </AppCheckbox>
              <AppCheckbox v-model="draft.multilinePaste" class="span-2">
                {{ t("settings.security.confirmMultilinePaste") }}
              </AppCheckbox>
            </NForm>
            <section
              class="settings-group"
              aria-labelledby="settings-system-detection"
            >
              <h2 id="settings-system-detection">
                {{ t("settings.sections.systemDetection") }}
              </h2>
              <SystemDetectionSettings v-model="draft.detectSshSystem" />
            </section>
            <section class="settings-group" aria-labelledby="settings-search">
              <h2 id="settings-search">{{ t("settings.sections.search") }}</h2>
              <SearchPreferences />
            </section>
          </div>
        </AppScrollArea>
      </div>
      <div
        v-if="visitedSections.has('security')"
        v-show="section === 'security'"
        class="settings-pane"
      >
        <AppScrollArea
          class="settings-scroll"
          :label="t('settings.sections.security')"
        >
          <div class="settings-section">
            <h1>{{ t("settings.sections.security") }}</h1>
            <VaultSecuritySettings ref="security" />
          </div>
        </AppScrollArea>
      </div>
      <div
        v-if="visitedSections.has('backup')"
        v-show="section === 'backup'"
        class="settings-pane"
      >
        <AppScrollArea
          class="settings-scroll"
          :label="t('settings.sections.backup')"
        >
          <div class="settings-section">
            <h1>{{ t("settings.sections.backup") }}</h1>
            <BackupPanel ref="backup" />
          </div>
        </AppScrollArea>
      </div>
      <div
        v-if="visitedSections.has('themes')"
        v-show="section === 'themes'"
        class="settings-pane"
      >
        <AppScrollArea
          class="settings-scroll"
          :label="t('terminal.form.theme')"
        >
          <div class="settings-section">
            <h1>{{ t("terminal.form.theme") }}</h1>
            <ThemeSettings v-model:settings="draft" />
          </div>
        </AppScrollArea>
      </div>
      <div
        v-if="visitedSections.has('shortcuts')"
        v-show="section === 'shortcuts'"
        class="settings-pane"
      >
        <AppScrollArea
          class="settings-scroll"
          :label="t('settings.sections.shortcuts')"
        >
          <div class="settings-section">
            <h1>{{ t("settings.sections.shortcuts") }}</h1>
            <ShortcutSettings v-model:settings="draft" />
          </div>
        </AppScrollArea>
      </div>
    </div>
  </section>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.settings-navigation {
  width: 190px;
  flex-shrink: 0;
  padding-top: 3px;
  overflow-y: auto;
  border-right: 1px solid var(--border);
  background: var(--surface);
}
.settings-content {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.settings-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
}
.settings-scroll {
  flex: 1;
}
.settings-section {
  width: 100%;
  max-width: 1200px;
  margin-inline: auto;
  padding: 24px 28px;
}
.settings-section > h1 {
  margin-bottom: 24px;
  font-size: 24px;
}
.settings-group {
  margin-top: 24px;
  padding-top: 24px;
  border-top: 1px solid var(--border);
}
.settings-group > h2 {
  margin-bottom: 16px;
  font-size: 15px;
}
.primary-color-row {
  display: flex;
  width: 100%;
  min-width: 0;
  gap: 8px;
}
.host-sort-field,
.punctuation-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}
.host-sort-field p,
.punctuation-field p {
  margin: 0;
}
.primary-color-row :deep(.n-color-picker) {
  flex: 1;
  min-width: 0;
}
@media (max-width: 760px) {
  .settings-section {
    padding: 20px;
  }
  .form-grid {
    grid-template-columns: minmax(0, 1fr);
  }
  .span-2 {
    grid-column: 1 / -1;
  }
}
</style>

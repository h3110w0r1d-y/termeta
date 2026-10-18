<script setup lang="ts">
import { t } from "../../i18n";
import { MoonOutline, SunnyOutline, DesktopOutline } from "@vicons/ionicons5";
import {
  NColorPicker,
  NButton,
  NTabs,
  NTabPane,
  NForm,
  NFormItem,
  type FormInst,
  type FormItemInst,
  type FormRules,
  NRadioGroup,
  NRadioButton,
  NIcon,
  type SelectRenderLabel,
} from "naive-ui";
import AppNumberField from "../../components/ui/AppNumberField.vue";
import AppCheckbox from "../../components/ui/AppCheckbox.vue";
import AppSelect from "../../components/ui/AppSelect.vue";
import SystemDetectionSettings from "./SystemDetectionSettings.vue";
import SearchPreferences from "./SearchPreferences.vue";
import { requiredText, integerRange } from "../../services/forms/rules";
import { computed, h, ref } from "vue";
import AppDialog from "../../components/ui/AppDialog.vue";
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
const tab = ref("general");
const primaryColorItem = ref<FormItemInst>();
function resetPrimaryColor() {
  draft.value.primaryColor = "";
  // 按钮直接更新共享字段后，使用表单项公开接口重新校验。
  void primaryColorItem.value
    ?.validate({ trigger: "change" })
    .catch(() => undefined);
}
const rules = computed<FormRules>(() => ({
  tabPosition: requiredText("settings.general.tabPosition"),
  tabDoubleClickAction: requiredText("settings.general.tabDoubleClickAction"),
  appearance: requiredText("settings.general.appearance"),
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
}));
</script>
<template>
  <AppDialog
    embedded
    :open="true"
    :title="t('settings.title')"
    wide
    class="embedded-settings"
  >
    <div class="settings-page">
      <div>
        <fieldset class="settings-body">
          <NTabs v-model:value="tab" type="line" animated>
            <NTabPane
              name="general"
              :tab="t('settings.sections.general')"
              display-directive="show:lazy"
            >
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
                        label: t('settings.appearance.system'),
                      },
                      { value: 'zh-CN', label: '简体中文' },
                      { value: 'en-US', label: 'English' },
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
                  :label="t('settings.general.appearance')"
                  path="appearance"
                >
                  <NRadioGroup
                    v-model:value="draft.appearance"
                    :aria-label="t('settings.general.appearance')"
                  >
                    <NRadioButton
                      value="dark"
                      :title="t('settings.appearance.dark')"
                    >
                      <span class="appearance-option">
                        <NIcon :size="16" aria-hidden="true">
                          <MoonOutline />
                        </NIcon>
                        <span class="appearance-label">
                          {{ t("settings.appearance.dark") }}
                        </span>
                      </span>
                    </NRadioButton>
                    <NRadioButton
                      value="light"
                      :title="t('settings.appearance.light')"
                    >
                      <span class="appearance-option">
                        <NIcon :size="16" aria-hidden="true">
                          <SunnyOutline />
                        </NIcon>
                        <span class="appearance-label">
                          {{ t("settings.appearance.light") }}
                        </span>
                      </span>
                    </NRadioButton>
                    <NRadioButton
                      value="system"
                      :title="t('settings.appearance.system')"
                    >
                      <span class="appearance-option">
                        <NIcon :size="16" aria-hidden="true">
                          <DesktopOutline />
                        </NIcon>
                        <span class="appearance-label">
                          {{ t("settings.appearance.system") }}
                        </span>
                      </span>
                    </NRadioButton>
                  </NRadioGroup>
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
                <NFormItem
                  :label="t('settings.terminal.font')"
                  path="fontFamily"
                >
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
                  <AppNumberField
                    v-model="draft.fontSize"
                    :min="10"
                    :max="32"
                  />
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
                <AppCheckbox v-model="draft.confirmClose" class="span-2">
                  {{ t("settings.security.confirmClose") }}
                </AppCheckbox>
                <AppCheckbox v-model="draft.multilinePaste" class="span-2">
                  {{ t("settings.security.confirmMultilinePaste") }}
                </AppCheckbox>
                <p class="muted span-2">
                  {{ t("settings.security.description") }}
                </p>
              </NForm>
            </NTabPane>
            <NTabPane
              name="system-detection"
              :tab="t('settings.sections.systemDetection')"
              display-directive="show:lazy"
            >
              <SystemDetectionSettings v-model="draft.detectSshSystem" />
            </NTabPane>
            <NTabPane
              name="search"
              :tab="t('settings.sections.search')"
              display-directive="show:lazy"
            >
              <SearchPreferences />
            </NTabPane>
            <NTabPane
              name="security"
              :tab="t('settings.sections.security')"
              display-directive="show:lazy"
            >
              <VaultSecuritySettings ref="security" />
            </NTabPane>
            <NTabPane
              name="backup"
              :tab="t('settings.sections.backup')"
              display-directive="show:lazy"
            >
              <BackupPanel ref="backup" />
            </NTabPane>
            <NTabPane
              name="themes"
              :tab="t('terminal.form.theme')"
              display-directive="show:lazy"
            >
              <ThemeSettings v-model:settings="draft" />
            </NTabPane>
            <NTabPane
              name="shortcuts"
              :tab="t('settings.sections.shortcuts')"
              display-directive="show:lazy"
            >
              <ShortcutSettings v-model:settings="draft" />
            </NTabPane>
          </NTabs>
        </fieldset>
      </div>
    </div>
  </AppDialog>
</template>

<style scoped>
.primary-color-row {
  display: flex;
  width: 100%;
  min-width: 0;
  gap: 8px;
}
.primary-color-row :deep(.n-color-picker) {
  flex: 1;
  min-width: 0;
}
.ui-label {
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 13px;
}
.settings-body {
  min-height: 340px;
  max-height: 58dvh;
  overflow: auto;
  padding: 2px;
}
.settings-body :deep(h3) {
  font-size: 13px;
  margin-top: 20px;
}

.appearance-option {
  display: inline-flex;
  align-items: center;
}
.appearance-label {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
fieldset.settings-body {
  border: 0;
  margin: 0;
  padding-inline: 0;
  min-width: 0;
}
.embedded-settings .settings-body {
  max-height: none;
  overflow: visible;
  min-height: 0;
  padding: 0;
}
.save-success {
  color: var(--accent);
  font-size: 13px;
}
.settings-page {
  width: 100%;
  padding: 12px 28px;
  max-width: 1540px;
  margin: 0 auto;
}
</style>

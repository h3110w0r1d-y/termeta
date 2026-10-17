<script setup lang="ts">
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
import { h, ref } from "vue";
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
const rules: FormRules = {
  tabPosition: requiredText("标签栏位置"),
  tabDoubleClickAction: requiredText("双击标签页动作"),
  appearance: requiredText("应用外观"),
  primaryColor: {
    trigger: ["change", "blur"],
    validator: (_, value: string) =>
      value === "" ||
      /^#[0-9a-f]{6}$/i.test(value) ||
      new Error("请输入6位HEX颜色"),
  },
  fontFamily: requiredText("终端字体"),
  fontSize: {
    required: true,
    type: "number",
    min: 10,
    max: 32,
    message: "字号须为10–32",
    trigger: ["blur", "change"],
  },
  scrollback: integerRange("滚屏行数", 1000, 50000),
};
</script>
<template>
  <AppDialog embedded :open="true" title="设置" wide class="embedded-settings">
    <div class="home-page">
      <div>
        <fieldset class="settings-body">
          <NTabs v-model:value="tab" type="line" animated>
            <NTabPane name="general" tab="常规" display-directive="show:lazy">
              <NForm
                ref="form"
                :model="draft"
                :rules="rules"
                label-placement="top"
                @submit.prevent
                class="form-grid"
              >
                <NFormItem label="标签栏位置" path="tabPosition">
                  <NRadioGroup
                    v-model:value="draft.tabPosition"
                    aria-label="标签栏位置"
                  >
                    <NRadioButton value="top">顶部</NRadioButton>
                    <NRadioButton value="left">左侧</NRadioButton>
                  </NRadioGroup>
                </NFormItem>
                <NFormItem label="双击标签页" path="tabDoubleClickAction">
                  <AppSelect
                    v-model="draft.tabDoubleClickAction"
                    :options="[
                      { value: 'none', label: '无动作' },
                      { value: 'close', label: '关闭标签页' },
                      { value: 'duplicate', label: '复制标签页' },
                      { value: 'split', label: '拆分会话' },
                    ]"
                  />
                </NFormItem>
                <NFormItem label="应用外观" path="appearance">
                  <NRadioGroup
                    v-model:value="draft.appearance"
                    aria-label="应用外观"
                  >
                    <NRadioButton value="dark" title="深色">
                      <span class="appearance-option">
                        <NIcon :size="16" aria-hidden="true">
                          <MoonOutline />
                        </NIcon>
                        <span class="appearance-label">深色</span>
                      </span>
                    </NRadioButton>
                    <NRadioButton value="light" title="浅色">
                      <span class="appearance-option">
                        <NIcon :size="16" aria-hidden="true">
                          <SunnyOutline />
                        </NIcon>
                        <span class="appearance-label">浅色</span>
                      </span>
                    </NRadioButton>
                    <NRadioButton value="system" title="跟随系统">
                      <span class="appearance-option">
                        <NIcon :size="16" aria-hidden="true">
                          <DesktopOutline />
                        </NIcon>
                        <span class="appearance-label">跟随系统</span>
                      </span>
                    </NRadioButton>
                  </NRadioGroup>
                </NFormItem>
                <NFormItem
                  ref="primaryColorItem"
                  label="主题色"
                  path="primaryColor"
                >
                  <div class="primary-color-row">
                    <NColorPicker
                      :value="draft.primaryColor || null"
                      :modes="['hex']"
                      :show-alpha="false"
                      aria-label="主题色"
                      @update:value="draft.primaryColor = $event ?? ''"
                    />
                    <NButton @click="resetPrimaryColor">恢复默认</NButton>
                  </div>
                </NFormItem>
                <NFormItem label="终端字体" path="fontFamily">
                  <AppSelect
                    v-model="draft.fontFamily"
                    :options="terminalFontOptions"
                    :render-label="renderFontLabel"
                  />
                </NFormItem>
                <NFormItem label="字号" path="fontSize">
                  <AppNumberField
                    v-model="draft.fontSize"
                    :min="10"
                    :max="32"
                  />
                </NFormItem>
                <NFormItem label="滚屏行数" path="scrollback">
                  <AppNumberField
                    v-model="draft.scrollback"
                    :min="1000"
                    :max="50000"
                  />
                </NFormItem>
                <AppCheckbox v-model="draft.confirmClose" class="span-2">
                  关闭可能正在执行命令的终端前确认
                </AppCheckbox>
                <AppCheckbox v-model="draft.multilinePaste" class="span-2">
                  多行粘贴前确认
                </AppCheckbox>
                <p class="muted span-2">
                  终端历史仅保留在当前窗口内存中。主机、信任、设置和凭据加密保存在本地；系统钥匙串仅缓存保险库主密钥。
                </p>
              </NForm>
            </NTabPane>
            <NTabPane
              name="system-detection"
              tab="SSH 系统识别"
              display-directive="show:lazy"
            >
              <SystemDetectionSettings v-model="draft.detectSshSystem" />
            </NTabPane>
            <NTabPane name="search" tab="搜索" display-directive="show:lazy">
              <SearchPreferences />
            </NTabPane>
            <NTabPane name="security" tab="安全" display-directive="show:lazy">
              <VaultSecuritySettings ref="security" />
            </NTabPane>
            <NTabPane
              name="backup"
              tab="备份 / 同步"
              display-directive="show:lazy"
            >
              <BackupPanel ref="backup" />
            </NTabPane>
            <NTabPane
              name="themes"
              tab="终端配色"
              display-directive="show:lazy"
            >
              <ThemeSettings v-model:settings="draft" />
            </NTabPane>
            <NTabPane
              name="shortcuts"
              tab="快捷键"
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
</style>

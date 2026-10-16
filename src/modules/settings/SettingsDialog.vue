<script setup lang="ts">
import {
  NTabs,
  NTabPane,
  NForm,
  NFormItem,
  type FormInst,
  type FormRules,
  NFlex,
  NH1,
  NText,
  NRadioGroup,
  NRadioButton,
  NIcon,
  type SelectRenderLabel,
} from "naive-ui";
import AppNumberField from "../../components/ui/AppNumberField.vue";
import AppCheckbox from "../../components/ui/AppCheckbox.vue";
import AppSelect from "../../components/ui/AppSelect.vue";
import SearchPreferences from "./SearchPreferences.vue";
import {
  requiredText,
  integerRange,
  validateForm,
} from "../../services/forms/rules";
import AppButton from "../../components/ui/AppButton.vue";
import { h, ref, watch } from "vue";
import AppDialog from "../../components/ui/AppDialog.vue";
import ThemeSettings from "./ThemeSettings.vue";
import ShortcutSettings from "./ShortcutSettings.vue";
import SecuritySettings from "./SecuritySettings.vue";
import { errorMessage } from "../../services/desktop/client";
import { clone } from "../../services/data/clone";
import { validateBindings } from "../../services/commands/registry";
import { terminalRegistry } from "../terminal/registry";
import { useSettings, defaultSettings } from "./store";
import { terminalFontFamily, terminalFontOptions } from "./fonts";
import type { AppSettings } from "./model";
// NSelect 同时用此渲染器显示菜单与当前值；字体声明按可见选项自动加载本地资源。
const renderFontLabel: SelectRenderLabel = (option) => {
  const name = typeof option.label === "string" ? option.label : "";
  return h("span", { style: { fontFamily: terminalFontFamily(name) } }, name);
};
const props = defineProps<{ open: boolean; embedded?: boolean }>(),
  emit = defineEmits<{ "update:open": [boolean] }>();
const store = useSettings(),
  draft = ref<AppSettings>(defaultSettings()),
  baseline = ref<AppSettings>(defaultSettings()),
  tab = ref("general"),
  error = ref(""),
  busy = ref(false),
  saved = ref(false);
function resetDraft() {
  if (busy.value) return;
  baseline.value = clone(store.value);
  draft.value = clone(store.value);
  error.value = "";
  saved.value = false;
}
watch(
  () => props.open,
  (open) => {
    if (open) resetDraft();
  },
  { immediate: true },
);
watch(
  draft,
  () => {
    saved.value = false;
  },
  { deep: true, flush: "sync" },
);
watch(
  () => store.value,
  (next) => {
    if (!props.open || busy.value) return;
    // 延迟加载或其他页面更新设置时，只同步用户未修改的字段。
    const merged = clone(draft.value);
    for (const key of Object.keys(next) as (keyof AppSettings)[]) {
      if (
        JSON.stringify(draft.value[key]) === JSON.stringify(baseline.value[key])
      ) {
        Object.assign(merged, { [key]: clone(next[key]) });
      }
    }
    baseline.value = clone(next);
    draft.value = merged;
  },
  { deep: true },
);
function cancel() {
  if (busy.value) return;
  if (props.embedded) resetDraft();
  else emit("update:open", false);
}
const form = ref<FormInst>();
const rules: FormRules = {
  tabPosition: requiredText("标签栏位置"),
  tabDoubleClickAction: requiredText("双击标签页动作"),
  appearance: requiredText("应用外观"),
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
async function save() {
  if (busy.value || !(await validateForm(form.value))) return;
  const snapshot = clone(draft.value);
  busy.value = true;
  error.value = "";
  saved.value = false;
  try {
    validateBindings(snapshot.keymap.bindings);
    if (
      !Number.isFinite(snapshot.fontSize) ||
      !Number.isInteger(snapshot.scrollback) ||
      snapshot.fontSize < 10 ||
      snapshot.fontSize > 32 ||
      snapshot.scrollback < 1000 ||
      snapshot.scrollback > 50000
    )
      throw new Error("字号须为 10–32，滚屏须为 1000–50000");
    await store.save(snapshot);
    baseline.value = clone(snapshot);
    terminalRegistry.apply();
    if (!props.embedded) emit("update:open", false);
    saved.value = JSON.stringify(draft.value) === JSON.stringify(snapshot);
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <AppDialog
    :embedded="embedded"
    :open="open"
    title="设置"
    description="搜索设置即时保存，其余设置保存后应用。终端配色独立于应用外观。"
    wide
    :locked="busy"
    :class="{ 'embedded-settings': embedded }"
    @update:open="emit('update:open', $event)"
  >
    <div :class="{ 'home-page': embedded }">
      <header v-if="embedded" class="page-heading">
        <div>
          <NH1>设置</NH1>
          <NText depth="3">搜索设置即时保存，其余修改后保存即可应用。</NText>
        </div>
      </header>
      <div>
        <fieldset class="settings-body" :disabled="busy">
          <NTabs v-model:value="tab" type="line" animated>
            <NTabPane
              name="general"
              tab="常规"
              :disabled="busy"
              display-directive="show:lazy"
            >
              <NForm
                ref="form"
                :model="draft"
                :rules="rules"
                label-placement="top"
                :disabled="busy"
                @submit.prevent="save"
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
                          <svg viewBox="0 0 24 24" fill="none">
                            <path
                              d="M20.9 13.3A9 9 0 0 1 10.7 3.1 9 9 0 1 0 20.9 13.3Z"
                              stroke="currentColor"
                              stroke-width="1.5"
                              stroke-linejoin="round"
                            />
                          </svg> </NIcon
                        ><span class="appearance-label">深色</span>
                      </span>
                    </NRadioButton>
                    <NRadioButton value="light" title="浅色">
                      <span class="appearance-option">
                        <NIcon :size="16" aria-hidden="true">
                          <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                          >
                            <circle cx="12" cy="12" r="4" />
                            <path
                              d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"
                            />
                          </svg> </NIcon
                        ><span class="appearance-label">浅色</span>
                      </span>
                    </NRadioButton>
                    <NRadioButton value="system" title="跟随系统">
                      <span class="appearance-option">
                        <NIcon :size="16" aria-hidden="true">
                          <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                          >
                            <rect x="3" y="4" width="18" height="13" rx="2" />
                            <path d="M12 17v4m-4 0h8" />
                          </svg> </NIcon
                        ><span class="appearance-label">跟随系统</span>
                      </span>
                    </NRadioButton>
                  </NRadioGroup>
                </NFormItem>
                <NFormItem label="终端字体" path="fontFamily"
                  ><AppSelect
                    v-model="draft.fontFamily"
                    :options="terminalFontOptions"
                    :render-label="renderFontLabel" /></NFormItem
                ><NFormItem label="字号" path="fontSize"
                  ><AppNumberField
                    v-model="draft.fontSize"
                    :min="10"
                    :max="32" /></NFormItem
                ><NFormItem label="滚屏行数" path="scrollback"
                  ><AppNumberField
                    v-model="draft.scrollback"
                    :min="1000"
                    :max="50000" /></NFormItem
                ><AppCheckbox v-model="draft.confirmClose" class="span-2"
                  >关闭可能正在执行命令的终端前确认</AppCheckbox
                ><AppCheckbox v-model="draft.multilinePaste" class="span-2"
                  >多行粘贴前确认</AppCheckbox
                >
                <p class="muted span-2">
                  终端历史仅保留在当前窗口内存中。主机、信任、设置和凭据加密保存在本地；系统钥匙串仅缓存保险库主密钥。
                </p>
              </NForm>
            </NTabPane>
            <NTabPane
              name="search"
              tab="搜索"
              :disabled="busy"
              display-directive="show:lazy"
              ><SearchPreferences
            /></NTabPane>
            <NTabPane
              name="themes"
              tab="终端配色"
              :disabled="busy"
              display-directive="show:lazy"
              ><ThemeSettings v-model:settings="draft"
            /></NTabPane>
            <NTabPane
              name="shortcuts"
              tab="快捷键"
              :disabled="busy"
              display-directive="show:lazy"
              ><ShortcutSettings v-model:settings="draft"
            /></NTabPane>
            <NTabPane
              name="security"
              tab="信任与历史"
              :disabled="busy"
              display-directive="show:lazy"
              ><SecuritySettings
            /></NTabPane>
          </NTabs>
        </fieldset>
      </div>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <p v-if="saved" class="save-success" role="status">设置已保存并应用。</p>
    </div>
    <template #footer>
      <NFlex justify="end">
        <AppButton :disabled="busy" @click="cancel">
          {{ embedded ? "放弃修改" : "取消" }}</AppButton
        ><AppButton class="primary" :disabled="busy" @click="save">
          {{ busy ? "保存中…" : "保存设置" }}
        </AppButton>
      </NFlex>
    </template></AppDialog
  >
</template>

<style scoped>
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

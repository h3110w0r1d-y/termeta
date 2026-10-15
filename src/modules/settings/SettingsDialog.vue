<script setup lang="ts">
import {
  NTabs,
  NTabPane,
  NForm,
  NFlex,
  NH1,
  NText,
  type SelectRenderLabel,
} from "naive-ui";
import AppNumberField from "../../components/ui/AppNumberField.vue";
import AppCheckbox from "../../components/ui/AppCheckbox.vue";
import AppSelect from "../../components/ui/AppSelect.vue";
import AppLabel from "../../components/ui/AppLabel.vue";
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
import { builtinSchemes } from "./themes";
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
async function save() {
  if (busy.value) return;
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
    description="设置成功保存后统一应用。终端配色独立于应用外观。"
    wide
    :locked="busy"
    :class="{ 'embedded-settings': embedded }"
    @update:open="emit('update:open', $event)"
  >
    <div :class="{ 'home-page': embedded }">
      <header v-if="embedded" class="page-heading">
        <div>
          <NH1>设置</NH1>
          <NText depth="3">让工作区适合你的习惯。修改后保存即可应用。</NText>
        </div>
      </header>
      <NForm :disabled="busy" @submit.prevent="save"
        ><fieldset class="settings-body" :disabled="busy">
          <NTabs v-model:value="tab" type="line" animated>
            <NTabPane
              name="general"
              tab="常规"
              :disabled="busy"
              display-directive="show:lazy"
            >
              <div class="form-grid">
                <AppLabel class="span-2"
                  >标签栏位置<AppSelect
                    v-model="draft.tabPosition"
                    :options="[
                      { value: 'top', label: '顶部标签栏' },
                      { value: 'left', label: '左侧标签栏' },
                    ]"
                  /><small class="muted"
                    >首页与 SFTP 固定在标签栏起始位置，SSH 会话依次排列。</small
                  ></AppLabel
                >
                <AppLabel
                  >应用外观<AppSelect
                    v-model="draft.appearance"
                    :options="[
                      { value: 'dark', label: '深色' },
                      { value: 'light', label: '浅色' },
                      { value: 'system', label: '跟随系统' },
                    ]" /></AppLabel
                ><AppLabel
                  >默认终端方案<AppSelect
                    v-model="draft.defaultSchemeId"
                    :options="
                      [...builtinSchemes, ...draft.customSchemes].map((s) => ({
                        value: s.id,
                        label: s.name,
                      }))
                    " /></AppLabel
                ><AppLabel class="span-2"
                  >终端字体<AppSelect
                    v-model="draft.fontFamily"
                    :options="terminalFontOptions"
                    :render-label="renderFontLabel" /></AppLabel
                ><AppLabel
                  >字号<AppNumberField
                    v-model="draft.fontSize"
                    :min="10"
                    :max="32" /></AppLabel
                ><AppLabel
                  >滚屏行数<AppNumberField
                    v-model="draft.scrollback"
                    :min="1000"
                    :max="50000" /></AppLabel
                ><AppCheckbox v-model="draft.confirmClose" class="span-2"
                  >关闭活动会话前确认</AppCheckbox
                ><AppCheckbox v-model="draft.multilinePaste" class="span-2"
                  >多行粘贴前确认</AppCheckbox
                >
                <p class="muted span-2">
                  终端历史仅保留在当前窗口内存中。主机、信任和设置保存在本地；密码和私钥口令仅由系统凭据库管理。
                </p>
              </div>
            </NTabPane>
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
          </NTabs></fieldset
      ></NForm>
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

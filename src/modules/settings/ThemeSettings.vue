<script setup lang="ts">
import { t } from "../../i18n";
import {
  NForm,
  NFormItem,
  type FormInst,
  type FormRules,
  NFlex,
  NGrid,
  NGridItem,
  NCollapse,
  NCollapseItem,
  NEmpty,
} from "naive-ui";
import AppCollapsible from "../../components/ui/AppCollapsible.vue";
import AppInput from "../../components/ui/AppInput.vue";
import AppTextarea from "../../components/ui/AppTextarea.vue";
import { requiredText, validateForm } from "../../services/forms/rules";
import AppButton from "../../components/ui/AppButton.vue";
import {
  computed,
  ref,
  shallowRef,
  watch,
  nextTick,
  onMounted,
  onBeforeUnmount,
  reactive,
} from "vue";
import { Terminal } from "@xterm/xterm";
import ColorField from "./ColorField.vue";
import SchemeThumbnail from "./SchemeThumbnail.vue";
import { applyTerminalTheme } from "../terminal/theme";
import type { Scheme } from "./model";
import {
  builtinSchemes,
  defaultScheme,
  defaultSchemeId,
  resolveScheme,
  parseScheme,
  downloadJson,
} from "./themes";
import { parseJson } from "../../services/data/json";
import { clone } from "../../services/data/clone";
import { confirmAction } from "../../components/ui/confirmation";
import { errorMessage } from "../../services/desktop/client";
import type { AppSettings } from "./model";
let syncingEditor = false;
const themeFile = ref<HTMLInputElement>();
const draft = defineModel<AppSettings>("settings", { required: true });
const error = ref(""),
  editor = ref<Scheme>(clone(defaultScheme)),
  previewHost = ref<HTMLElement>(),
  preview = shallowRef<Terminal>(),
  importText = ref(""),
  imported = ref<Scheme>(),
  schemeSearch = ref(""),
  expanded = ref<string[]>([]),
  indexed = ref("");
const formModel = reactive({ editor, indexed });
const schemes = computed(() =>
  [...builtinSchemes, ...draft.value.customSchemes].filter((s) =>
    s.name.toLowerCase().includes(schemeSearch.value.toLowerCase()),
  ),
);
const scrollbarKeys = ["normal", "hover", "active"] as const;
const colorLabels = computed(() => ({
  foreground: t("themes.colors.foreground"),
  background: t("themes.colors.background"),
  cursor: t("themes.colors.cursor"),
  cursorAccent: t("themes.colors.cursorAccent"),
  selectionBackground: t("themes.colors.selectionBackground"),
  selectionForeground: t("themes.colors.selectionForeground"),
  selectionInactiveBackground: t("themes.colors.selectionInactiveBackground"),
  normal: t("themes.colors.normal"),
  hover: t("themes.colors.hover"),
  active: t("themes.colors.active"),
}));
const contrast = computed(() => {
  const luminance = (hex: string) => {
    const values = [1, 3, 5]
      .map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
      .map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
    return values[0]! * 0.2126 + values[1]! * 0.7152 + values[2]! * 0.0722;
  };
  const a = luminance(editor.value.colors.foreground),
    b = luminance(editor.value.colors.background);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
});
const baseKeys = [
  "foreground",
  "background",
  "cursor",
  "cursorAccent",
  "selectionBackground",
  "selectionForeground",
  "selectionInactiveBackground",
] as const;
onMounted(() => {
  editScheme(
    resolveScheme(draft.value.defaultSchemeId, draft.value.customSchemes),
  );
});
watch(
  [expanded, previewHost],
  async () => {
    // 折叠内容首次显示后才初始化预览，后续收起保留同一实例。
    if (!expanded.value.includes("customize")) return;
    await nextTick();
    makePreview();
  },
  { flush: "post" },
);
watch(
  () => draft.value.defaultSchemeId,
  () =>
    editScheme(
      resolveScheme(draft.value.defaultSchemeId, draft.value.customSchemes),
    ),
);
function selectScheme(s: Scheme) {
  imported.value = undefined;
  if (draft.value.defaultSchemeId === s.id) editScheme(s);
  else draft.value.defaultSchemeId = s.id;
}
watch(
  editor,
  () => {
    if (preview.value && previewHost.value)
      applyTerminalTheme(preview.value, previewHost.value, editor.value);
  },
  { deep: true },
);
function makePreview() {
  if (!previewHost.value || preview.value) return;
  const term = new Terminal({
    // 粗体仅改变字重，避免普通 ANSI 色被自动替换为高亮色。
    drawBoldTextInBrightColors: false,
    cols: 65,
    rows: 9,
    fontSize: 12,
    disableStdin: true,
  });
  applyTerminalTheme(term, previewHost.value, editor.value);
  term.open(previewHost.value);
  preview.value = term;
  term.write(
    `Termeta ${t("themes.preview.label")} · 中文 / UTF-8 / 🖥\r\n\x1b[1mSSH workspace\x1b[0m  user@server ~\r\n`,
  );
  for (let i = 0; i < 16; i++) term.write(`\x1b[48;5;${i}m  \x1b[0m`);
  term.write("\r\n");
  for (let i = 16; i < 256; i++) term.write(`\x1b[48;5;${i}m \x1b[0m`);
  term.write(
    "\r\n\x1b[38;2;100;200;240mTruecolor RGB\x1b[0m  cursor ▌ / selection",
  );
}
onBeforeUnmount(() => preview.value?.dispose());
function editScheme(s: Scheme) {
  syncingEditor = true;
  editor.value = clone(s);
  editor.value.colors.scrollbar ??= {
    normal: "#8894a860",
    hover: "#8894a899",
    active: "#8894a8cc",
  };
  indexed.value = JSON.stringify(s.colors.indexedOverrides ?? {}, null, 2);
  if (s.source === "builtin") {
    editor.value.id = crypto.randomUUID();
    editor.value.name += t("themes.copySuffix");
    editor.value.source = "user";
  }
  void nextTick(() => {
    syncingEditor = false;
  });
}
const form = ref<FormInst>();
const rules: FormRules = {
  "editor.name": requiredText("themes.form.name"),
  indexed: {
    trigger: ["input", "blur"],
    validator: () => {
      try {
        const next = clone(editor.value);
        next.colors.indexedOverrides = parseJson(
          indexed.value || "{}",
        ) as Record<string, string>;
        parseScheme(JSON.stringify(next));
        return true;
      } catch (e) {
        return new Error(errorMessage(e));
      }
    },
  },
};
async function saveScheme() {
  if (!(await validateForm(form.value))) return;
  try {
    const candidate = clone(editor.value);
    candidate.colors.indexedOverrides = parseJson(
      indexed.value || "{}",
    ) as Record<string, string>;
    const checked = parseScheme(JSON.stringify(candidate));
    checked.id = editor.value.id;
    checked.source = "user";
    const at = draft.value.customSchemes.findIndex((s) => s.id === checked.id);
    if (at < 0) {
      if (draft.value.customSchemes.length >= 64)
        throw new Error(t("themes.validation.customLimit"));
      draft.value.customSchemes.push(checked);
    } else draft.value.customSchemes[at] = checked;
    draft.value.defaultSchemeId = checked.id;
    imported.value = undefined;
    error.value = "";
  } catch (e) {
    error.value = errorMessage(e);
  }
}
watch(
  [editor, indexed],
  () => {
    if (
      !syncingEditor &&
      expanded.value.includes("customize") &&
      !imported.value
    )
      void saveScheme();
  },
  { deep: true },
);
async function deleteScheme() {
  if (editor.value.source === "builtin") return;
  if (
    !(await confirmAction(
      t("themes.confirmDelete", {
        name: defaultScheme.name,
      }),
    ))
  )
    return;
  const id = editor.value.id;
  draft.value.customSchemes = draft.value.customSchemes.filter(
    (s) => s.id !== id,
  );
  if (draft.value.defaultSchemeId === id)
    draft.value.defaultSchemeId = defaultSchemeId;
  for (const [host, scheme] of Object.entries(draft.value.hostSchemes))
    if (scheme === id) delete draft.value.hostSchemes[host];
  editScheme(defaultScheme);
}
async function importFile(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  try {
    if (file.size > 1048576) throw new Error(t("themes.validation.importSize"));
    importText.value = await file.text();
    imported.value = parseScheme(importText.value);
    editScheme(imported.value);
    expanded.value = ["customize"];
  } catch (e) {
    error.value = errorMessage(e);
  }
}
async function exportTheme() {
  try {
    await downloadJson(editor.value.name + ".json", editor.value);
  } catch (e) {
    error.value = errorMessage(e);
  }
}
function confirmImport() {
  if (imported.value) {
    const next = clone(imported.value);
    if (
      [...builtinSchemes, ...draft.value.customSchemes].some(
        (s) => s.name === next.name,
      )
    )
      next.name += t("themes.importSuffix");
    if (draft.value.customSchemes.length >= 64) {
      error.value = t("themes.validation.customLimit");
      return;
    }
    draft.value.customSchemes.push(next);
    selectScheme(next);
    imported.value = undefined;
  }
}
</script>
<template>
  <div class="theme-layout">
    <NFlex align="center" :wrap="false">
      <AppInput
        v-model="schemeSearch"
        :placeholder="t('themes.search.placeholder')"
        :aria-label="t('themes.search.label')"
        style="flex: 1; min-width: 0"
      />
      <AppButton @click="themeFile?.click()">
        {{ t("themes.actions.importJson") }}
      </AppButton>
      <input
        ref="themeFile"
        hidden
        type="file"
        accept=".json,application/json"
        @change="importFile"
      />
    </NFlex>
    <NGrid
      cols="1 420:2 780:3 1040:4"
      responsive="self"
      :x-gap="12"
      :y-gap="12"
    >
      <NGridItem v-for="s in schemes" :key="s.id">
        <AppButton
          class="scheme-card"
          :variant="draft.defaultSchemeId === s.id ? 'primary' : 'default'"
          :aria-pressed="draft.defaultSchemeId === s.id"
          @click="selectScheme(s)"
        >
          <span class="scheme-card-content">
            <SchemeThumbnail :scheme="s" />
            <span class="scheme-name" :title="s.name">{{ s.name }}</span>
          </span>
        </AppButton>
      </NGridItem>
    </NGrid>
    <NEmpty
      v-if="!schemes.length"
      :description="t('themes.search.noResults')"
    />
    <NCollapse v-model:expanded-names="expanded">
      <NCollapseItem
        name="customize"
        :title="t('themes.actions.customize')"
        display-directive="show"
      >
        <NForm
          ref="form"
          :model="formModel"
          :rules="rules"
          label-placement="top"
          @submit.prevent="saveScheme"
        >
          <NFlex vertical :size="16">
            <p class="muted">
              {{ t("themes.form.builtinHint") }}
            </p>
            <NFormItem :label="t('themes.form.name')" path="editor.name">
              <AppInput v-model="formModel.editor.name" maxlength="80" />
            </NFormItem>
            <div ref="previewHost" class="theme-preview" />
            <p v-if="contrast < 4.5" class="error">
              {{
                t("themes.contrastWarning", {
                  ratio: contrast.toFixed(2),
                })
              }}
            </p>
            <NGrid cols="1 560:2" responsive="self" :x-gap="10" :y-gap="10">
              <NGridItem v-for="key in baseKeys" :key="key">
                <ColorField
                  v-model="formModel.editor.colors[key]"
                  :label="colorLabels[key]"
                  :path="`editor.colors.${key}`"
                  :optional="key === 'selectionForeground'"
                  :reset="defaultScheme.colors[key]"
                />
              </NGridItem>
            </NGrid>
            <AppCollapsible :title="t('themes.form.ansi')">
              <NGrid cols="1 560:2" responsive="self" :x-gap="10" :y-gap="10">
                <NGridItem v-for="(_, i) in editor.colors.ansi" :key="i">
                  <ColorField
                    v-model="formModel.editor.colors.ansi[i]"
                    :label="'ANSI ' + i"
                    :path="`editor.colors.ansi.${i}`"
                    :reset="defaultScheme.colors.ansi[i]"
                  />
                </NGridItem>
              </NGrid>
            </AppCollapsible>
            <AppCollapsible :title="t('themes.form.scrollbar')">
              <NGrid
                v-if="formModel.editor.colors.scrollbar"
                cols="1 560:2"
                responsive="self"
                :x-gap="10"
                :y-gap="10"
              >
                <NGridItem v-for="key in scrollbarKeys" :key="key">
                  <ColorField
                    v-model="formModel.editor.colors.scrollbar[key]"
                    :label="colorLabels[key]"
                    :path="`editor.colors.scrollbar.${key}`"
                    optional
                  />
                </NGridItem>
              </NGrid>
            </AppCollapsible>
            <NFormItem :label="t('themes.form.extendedColors')" path="indexed">
              <AppTextarea
                v-model="formModel.indexed"
                rows="3"
                spellcheck="false"
              />
            </NFormItem>
            <NFlex>
              <AppButton @click="exportTheme">
                {{ t("themes.actions.exportJson") }}
              </AppButton>
              <AppButton
                class="danger"
                :disabled="!draft.customSchemes.some((s) => s.id === editor.id)"
                @click="deleteScheme"
              >
                {{ t("themes.actions.deleteCustom") }}
              </AppButton>
              <AppButton
                @click="
                  editScheme(
                    resolveScheme(draft.defaultSchemeId, draft.customSchemes),
                  )
                "
              >
                {{ t("themes.actions.undo") }}
              </AppButton>
              <AppButton v-if="imported" @click="confirmImport">
                {{ t("themes.actions.confirmImport") }}
              </AppButton>
            </NFlex>
          </NFlex>
        </NForm>
      </NCollapseItem>
    </NCollapse>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
  </div>
</template>

<style scoped>
.theme-layout {
  display: flex;
  flex-direction: column;
  gap: 20px;
  min-width: 0;
}
.scheme-card {
  width: 100%;
  height: auto;
  padding: 8px 12px;
}
.scheme-card :deep(.n-button__content) {
  width: 100%;
  min-width: 0;
}
.scheme-card-content {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-width: 0;
}
.scheme-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}
.theme-preview {
  height: 170px;
  overflow: hidden;
  margin: 12px 0;
}
</style>

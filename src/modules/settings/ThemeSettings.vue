<script setup lang="ts">
import {
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
import AppLabel from "../../components/ui/AppLabel.vue";
import AppButton from "../../components/ui/AppButton.vue";
import {
  computed,
  ref,
  shallowRef,
  watch,
  nextTick,
  onMounted,
  onBeforeUnmount,
} from "vue";
import { Terminal } from "@xterm/xterm";
import ColorField from "./ColorField.vue";
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
const schemes = computed(() =>
  [...builtinSchemes, ...draft.value.customSchemes].filter((s) =>
    s.name.toLowerCase().includes(schemeSearch.value.toLowerCase()),
  ),
);
const scrollbarKeys = ["normal", "hover", "active"] as const;
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
watch([() => draft.value.defaultSchemeId, () => draft.value], () =>
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
    "Termvexa 配色预览 · 中文 / UTF-8 / 🖥\r\n\x1b[1mSSH workspace\x1b[0m  user@server ~\r\n",
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
  editor.value = clone(s);
  editor.value.colors.scrollbar ??= {
    normal: "#8894a860",
    hover: "#8894a899",
    active: "#8894a8cc",
  };
  indexed.value = JSON.stringify(s.colors.indexedOverrides ?? {}, null, 2);
  if (s.source === "builtin") {
    editor.value.id = crypto.randomUUID();
    editor.value.name += " 副本";
    editor.value.source = "user";
  }
}
function saveScheme() {
  try {
    editor.value.colors.indexedOverrides = parseJson(
      indexed.value || "{}",
    ) as Record<string, string>;
    const checked = parseScheme(JSON.stringify(editor.value));
    checked.id = editor.value.id;
    checked.source = "user";
    const at = draft.value.customSchemes.findIndex((s) => s.id === checked.id);
    if (at < 0) {
      if (draft.value.customSchemes.length >= 64)
        throw new Error("自定义方案最多 64 个");
      draft.value.customSchemes.push(checked);
    } else draft.value.customSchemes[at] = checked;
    draft.value.defaultSchemeId = checked.id;
    imported.value = undefined;
    error.value = "";
  } catch (e) {
    error.value = errorMessage(e);
  }
}
async function deleteScheme() {
  if (editor.value.source === "builtin") return;
  if (
    !(await confirmAction(
      `删除自定义方案？主机和终端引用将跟随全局默认；若删除的是全局默认，将改用 ${defaultScheme.name}。`,
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
    if (file.size > 1048576) throw new Error("文件超过 1 MiB");
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
      next.name += "（导入副本）";
    if (draft.value.customSchemes.length >= 64) {
      error.value = "自定义方案最多 64 个";
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
        placeholder="搜索配色…"
        aria-label="搜索终端配色"
        style="flex: 1; min-width: 0"
      />
      <AppButton @click="themeFile?.click()">导入 JSON</AppButton>
      <input
        ref="themeFile"
        hidden
        type="file"
        accept=".json,application/json"
        @change="importFile"
      />
    </NFlex>
    <NGrid
      cols="1 420:2 630:3 840:4 1050:5"
      responsive="self"
      :x-gap="12"
      :y-gap="12"
    >
      <NGridItem v-for="s in schemes" :key="s.id">
        <AppButton
          class="scheme-card"
          :variant="draft.defaultSchemeId === s.id ? 'primary' : 'default'"
          :aria-pressed="draft.defaultSchemeId === s.id"
          :style="{
            background: s.colors.background,
            color: s.colors.foreground,
          }"
          @click="selectScheme(s)"
        >
          <NFlex vertical :size="4" align="center">
            <span
              >{{ draft.defaultSchemeId === s.id ? "✓ " : ""
              }}{{ s.name }}</span
            >
            <span class="swatches" aria-hidden="true">
              <i
                v-for="(c, index) in s.colors.ansi.slice(1, 7)"
                :key="index"
                :style="{ background: c }"
              />
            </span>
          </NFlex>
        </AppButton>
      </NGridItem>
    </NGrid>
    <NEmpty v-if="!schemes.length" description="没有匹配的配色方案" />
    <NCollapse v-model:expanded-names="expanded">
      <NCollapseItem
        name="customize"
        title="自定义当前配色"
        display-directive="show"
      >
        <NFlex vertical :size="16">
          <p class="muted">
            内置方案编辑后保存为自定义副本。修改将在保存设置后应用。
          </p>
          <AppLabel
            >方案名称<AppInput v-model="editor.name" maxlength="80"
          /></AppLabel>
          <div ref="previewHost" class="theme-preview" />
          <p v-if="contrast < 4.5" class="error">
            前景 / 背景对比度 {{ contrast.toFixed(2) }}:1，低于
            4.5:1。保存时保留你的选色。
          </p>
          <div class="color-grid">
            <ColorField
              v-for="key in baseKeys"
              :key="key"
              v-model="editor.colors[key]"
              :label="key"
              :reset="defaultScheme.colors[key]"
            />
          </div>
          <AppCollapsible title="ANSI 16 色（取色 / HEX / RGB）">
            <div class="color-grid">
              <ColorField
                v-for="(_, i) in editor.colors.ansi"
                :key="i"
                v-model="editor.colors.ansi[i]"
                :label="'ANSI ' + i"
                :reset="defaultScheme.colors.ansi[i]"
              />
            </div>
          </AppCollapsible>
          <AppCollapsible title="滚动条辅助颜色">
            <div v-if="editor.colors.scrollbar" class="color-grid">
              <ColorField
                v-for="key in scrollbarKeys"
                :key="key"
                v-model="editor.colors.scrollbar[key]"
                :label="key"
              />
            </div>
          </AppCollapsible>
          <AppLabel
            >索引色覆盖（16–255，HEX JSON）<AppTextarea
              v-model="indexed"
              rows="3"
              spellcheck="false"
          /></AppLabel>
          <NFlex>
            <AppButton @click="exportTheme">导出 JSON</AppButton>
            <AppButton
              class="danger"
              :disabled="!draft.customSchemes.some((s) => s.id === editor.id)"
              @click="deleteScheme"
              >删除自定义</AppButton
            >
            <AppButton
              @click="
                editScheme(
                  resolveScheme(draft.defaultSchemeId, draft.customSchemes),
                )
              "
              >撤销编辑</AppButton
            >
            <AppButton v-if="imported" @click="confirmImport"
              >确认导入副本</AppButton
            >
          </NFlex>
          <NFlex justify="end">
            <AppButton class="primary" @click="saveScheme"
              >保存为自定义</AppButton
            >
          </NFlex>
        </NFlex>
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
.swatches {
  display: inline-flex;
  gap: 3px;
}
.swatches i {
  width: 14px;
  height: 6px;
  display: inline-block;
}
.theme-preview {
  height: 170px;
  overflow: hidden;
  margin: 12px 0;
}
.color-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 10px;
}
</style>

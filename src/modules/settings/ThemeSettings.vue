<script setup lang="ts">
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
import type { Scheme } from "./model";
import {
  builtinSchemes,
  resolveScheme,
  toXtermTheme,
  parseScheme,
  downloadJson,
} from "./themes";
import { parseJson } from "../../services/data/json";
import { clone } from "../../services/data/clone";
import { confirmAction } from "../../components/ui/confirmation";
import { errorMessage } from "../../services/desktop/client";
import type { AppSettings } from "./model";
const draft = defineModel<AppSettings>("settings", { required: true });
const error = ref(""),
  editor = ref<Scheme>(clone(builtinSchemes[0]!)),
  previewHost = ref<HTMLElement>(),
  preview = shallowRef<Terminal>(),
  importText = ref(""),
  imported = ref<Scheme>(),
  schemeSearch = ref(""),
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
onMounted(async () => {
  editScheme(
    resolveScheme(draft.value.defaultSchemeId, draft.value.customSchemes),
  );
  await nextTick();
  makePreview();
});
watch(
  editor,
  () => {
    if (preview.value) preview.value.options.theme = toXtermTheme(editor.value);
  },
  { deep: true },
);
function makePreview() {
  if (!previewHost.value || preview.value) return;
  const term = new Terminal({
    cols: 65,
    rows: 9,
    fontSize: 12,
    theme: toXtermTheme(editor.value),
    disableStdin: true,
  });
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
    error.value = "";
  } catch (e) {
    error.value = errorMessage(e);
  }
}
async function deleteScheme() {
  if (editor.value.source === "builtin") return;
  if (
    !(await confirmAction(
      "删除自定义方案？全局和当前终端引用将改为 Termvexa Dark。",
    ))
  )
    return;
  const id = editor.value.id;
  draft.value.customSchemes = draft.value.customSchemes.filter(
    (s) => s.id !== id,
  );
  if (draft.value.defaultSchemeId === id) draft.value.defaultSchemeId = "dark";
  for (const [host, scheme] of Object.entries(draft.value.hostSchemes))
    if (scheme === id) delete draft.value.hostSchemes[host];
  editScheme(builtinSchemes[0]!);
}
async function importFile(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  try {
    if (file.size > 1048576) throw new Error("文件超过 1 MiB");
    importText.value = await file.text();
    imported.value = parseScheme(importText.value);
    editScheme(imported.value);
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
    imported.value = undefined;
  }
}
</script>
<template>
  <div class="theme-layout">
    <section>
      <input v-model="schemeSearch" placeholder="搜索配色…" /><button
        v-for="s in schemes"
        :key="s.id"
        class="scheme-card"
        :style="{
          background: s.colors.background,
          color: s.colors.foreground,
        }"
        @click="editScheme(s)"
      >
        {{ s.name
        }}<span class="swatches"
          ><i
            v-for="c in s.colors.ansi.slice(1, 7)"
            :key="c"
            :style="{ background: c }"
        /></span></button
      ><label class="file-button"
        >导入 JSON<input
          type="file"
          accept=".json,application/json"
          @change="importFile"
      /></label>
      <p class="muted">
        支持 Termvexa v1 / xterm ITheme；缺项以 Dark 补全，导入后请核对预览。
      </p>
      <button v-if="imported" class="primary" @click="confirmImport">
        确认导入副本
      </button>
    </section>
    <section>
      <label>方案名称<input v-model="editor.name" maxlength="80" /></label>
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
          :reset="builtinSchemes[0]!.colors[key]"
        />
      </div>
      <details>
        <summary>ANSI 16 色（取色 / HEX / RGB）</summary>
        <div class="color-grid">
          <ColorField
            v-for="(_, i) in editor.colors.ansi"
            :key="i"
            v-model="editor.colors.ansi[i]"
            :label="'ANSI ' + i"
            :reset="builtinSchemes[0]!.colors.ansi[i]"
          />
        </div>
      </details>
      <details>
        <summary>滚动条辅助颜色</summary>
        <div v-if="editor.colors.scrollbar" class="color-grid">
          <ColorField
            v-for="key in scrollbarKeys"
            :key="key"
            v-model="editor.colors.scrollbar[key]"
            :label="key"
          />
        </div>
      </details>
      <label
        >索引色覆盖（16–255，HEX JSON）<textarea
          v-model="indexed"
          rows="3"
          spellcheck="false"
        />
      </label>
      <div class="button-row">
        <button @click="saveScheme">保存为自定义</button
        ><button @click="exportTheme">导出 JSON</button
        ><button class="danger" @click="deleteScheme">删除</button
        ><button
          @click="
            editScheme(
              resolveScheme(draft.defaultSchemeId, draft.customSchemes),
            )
          "
        >
          撤销编辑
        </button>
      </div>
    </section>
  </div>
  <p v-if="error" class="error" role="alert">{{ error }}</p>
</template>

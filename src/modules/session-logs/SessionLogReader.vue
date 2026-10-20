<script setup lang="ts">
import { t, formatDate } from "@/i18n";
import { nextTick, onBeforeUnmount, ref, watch } from "vue";
import { NAlert, NEmpty, NFlex, NSpin, NText } from "naive-ui";
import AppButton from "@/components/ui/AppButton.vue";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { SessionLogSummary } from "@/services/desktop/types";
import { sessionLogEndpoint, sessionLogProtocol } from "./presentation";
import { readTerminalLogSnapshot, type TerminalLogSnapshot } from "./snapshot";
import { mountLogTerminal } from "./viewer";
const props = defineProps<{ log: SessionLogSummary }>();
const output = ref<HTMLElement>();
const loading = ref(false);
const loaded = ref(false);
const empty = ref(false);
const error = ref("");
const hasScreen = ref(false);
const showScreen = ref(false);
const richContent = ref(false);
let snapshot: TerminalLogSnapshot | undefined;
let viewer: ReturnType<typeof mountLogTerminal> | undefined;
let generation = 0;
function clearOutput() {
  viewer?.dispose();
  viewer = undefined;
  output.value?.replaceChildren();
  output.value?.style.removeProperty("background-color");
}
async function toggleScreen() {
  if (!snapshot || !output.value || loading.value) return;
  const current = generation;
  loading.value = true;
  error.value = "";
  try {
    showScreen.value = !showScreen.value;
    clearOutput();
    output.value.style.backgroundColor = snapshot.background;
    const host = document.createElement("div");
    host.className = "log-terminal";
    output.value.append(host);
    viewer = mountLogTerminal(host, snapshot, showScreen.value);
    await viewer.ready;
  } catch (cause) {
    if (current === generation) error.value = errorMessage(cause);
  } finally {
    if (current === generation) loading.value = false;
  }
}
// 正文只驻留独立只读终端或文本节点，不进入 Store、响应式对象或 HTML 解释路径。
async function load() {
  const current = ++generation;
  loading.value = true;
  loaded.value = false;
  error.value = "";
  clearOutput();
  snapshot = undefined;
  hasScreen.value = false;
  showScreen.value = false;
  richContent.value = false;
  try {
    const log = await desktop.readSessionLog(props.log.id);
    if (current !== generation) return;
    snapshot = readTerminalLogSnapshot(log.content);
    richContent.value = snapshot !== undefined;
    hasScreen.value = snapshot?.screen !== undefined;
    empty.value = !log.content.length;
    loaded.value = true;
    await nextTick();
    if (current !== generation || !output.value) return;
    if (snapshot) {
      output.value.style.backgroundColor = snapshot.background;
      const host = document.createElement("div");
      host.className = "log-terminal";
      output.value.append(host);
      viewer = mountLogTerminal(host, snapshot, false);
      await viewer.ready;
    } else {
      // 旧记录没有颜色属性，只按纯文本展示，不能将其中的转义序列作为命令重放。
      const text = document.createElement("pre");
      text.className = "legacy-log";
      text.textContent = log.content;
      output.value.append(text);
    }
  } catch (cause) {
    if (current === generation) error.value = errorMessage(cause);
  } finally {
    if (current === generation) loading.value = false;
  }
}
watch(() => props.log.id, load, { immediate: true });
onBeforeUnmount(() => {
  generation++;
  clearOutput();
  snapshot = undefined;
});
</script>
<template>
  <section class="session-log-reader">
    <header class="log-heading">
      <NFlex vertical :size="4">
        <strong class="selectable-text">{{ log.name }}</strong>
        <NText depth="3" class="selectable-text">
          {{
            [sessionLogProtocol(log.protocol), sessionLogEndpoint(log)]
              .filter(Boolean)
              .join(" · ")
          }}
        </NText>
        <NText depth="3">
          {{
            t("sessionLogs.timeRange", {
              started: formatDate(log.startedAt),
              closed: formatDate(log.closedAt),
            })
          }}
        </NText>
      </NFlex>
      <NFlex>
        <AppButton v-if="hasScreen" :disabled="loading" @click="toggleScreen">
          {{
            showScreen
              ? t("sessionLogs.showHistory")
              : t("sessionLogs.showScreen")
          }}
        </AppButton>
        <AppButton :disabled="loading" @click="load">
          {{ t("common.actions.refresh") }}
        </AppButton>
      </NFlex>
    </header>
    <NAlert v-if="error" type="error">{{ error }}</NAlert>
    <NSpin class="log-loading" :show="loading">
      <NEmpty
        v-if="loaded && empty"
        class="empty-log"
        :description="t('sessionLogs.emptyContent')"
      />
      <div
        ref="output"
        v-show="loaded && !empty"
        class="log-content selectable-text"
        :class="{ 'terminal-content': richContent }"
        tabindex="0"
        :aria-label="t('sessionLogs.content')"
      />
    </NSpin>
  </section>
</template>
<style scoped>
.session-log-reader {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.log-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 20px;
  border-bottom: 1px solid var(--border);
}
.log-loading {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.log-loading :deep(.n-spin-content) {
  height: 100%;
  min-height: 0;
}
.log-content {
  box-sizing: border-box;
  height: 100%;
  margin: 0;
  padding: 20px;
  overflow: auto;
  font-family: "DejaVu Sans Mono", monospace;
  font-size: 13px;
  line-height: 1.5;
  tab-size: 8;
}
.log-content :deep(.legacy-log) {
  margin: 0;
  font: inherit;
  white-space: pre;
}
.log-content.terminal-content {
  overflow: hidden;
}
.log-content :deep(.log-terminal) {
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.log-content :deep(.log-screen-viewport) {
  overflow: auto;
  overscroll-behavior: contain;
}
.log-content :deep(.log-screen-surface) {
  position: relative;
}
.log-content :deep(.xterm) {
  height: 100%;
}
.empty-log {
  padding: 48px 20px;
}
</style>

<script setup lang="ts">
import type { Scheme } from "./model";
defineProps<{ scheme: Scheme }>();
</script>

<template>
  <!-- 用轻量色块预览配色，不为每张卡片创建终端实例。 -->
  <span
    class="scheme-thumbnail"
    aria-hidden="true"
    :style="{
      '--sample-background': scheme.colors.background,
      '--sample-foreground': scheme.colors.foreground,
      '--sample-selection': scheme.colors.selectionBackground,
      '--sample-selected-text':
        scheme.colors.selectionForeground ?? scheme.colors.foreground,
      '--sample-cursor': scheme.colors.cursor,
      '--sample-green': scheme.colors.ansi[2],
    }"
  >
    <span class="sample-text" />
    <span class="sample-selection">
      <span class="selection-first" />
      <span class="selection-second" />
    </span>
    <span class="sample-bottom">
      <span class="sample-green" />
      <span class="bottom-first" />
      <span class="bottom-second" />
      <span class="sample-cursor" />
    </span>
    <span class="sample-palette">
      <span
        v-for="(color, index) in scheme.colors.ansi.slice(1, 7)"
        :key="index"
        :style="{ backgroundColor: color }"
      />
    </span>
  </span>
</template>

<style scoped>
.scheme-thumbnail {
  position: relative;
  display: block;
  flex: 0 0 72px;
  width: 72px;
  height: 48px;
  padding: 5px 3px;
  overflow: hidden;
  border: 1px solid var(--sample-selection);
  border-radius: 6px;
  background: var(--sample-background);
}
.sample-text,
.selection-first,
.selection-second,
.sample-green,
.bottom-first,
.bottom-second {
  display: block;
  height: 4px;
  border-radius: 40px;
  background: var(--sample-foreground);
}
.sample-text {
  margin: 0 2px 3px;
}
.sample-selection {
  display: flex;
  justify-content: space-between;
  height: 8px;
  padding: 2px;
  margin-bottom: 3px;
  border-radius: 2px;
  background: var(--sample-selection);
}
.selection-first {
  width: 36px;
  background: var(--sample-selected-text);
}
.selection-second {
  width: 22px;
  background: var(--sample-selected-text);
}
.sample-bottom {
  display: flex;
  align-items: center;
  gap: 2px;
  margin: 0 2px;
}
.sample-green {
  width: 16px;
  background: var(--sample-green);
}
.bottom-first {
  width: 17px;
}
.bottom-second {
  width: 10px;
}
.sample-cursor {
  width: 3px;
  height: 6px;
  background: var(--sample-cursor);
}
/* 六色条始终可见，不依赖悬停，也不参与上方文字布局。 */
.sample-palette {
  position: absolute;
  inset: auto 0 0;
  display: flex;
  height: 6px;
}
.sample-palette > span {
  flex: 1;
}
</style>

<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import { NButton, NDropdown, NIcon, NInput, type InputInst } from "naive-ui";
import { IconArrowRight, IconChevronRight, IconDots } from "@tabler/icons-vue";
import { t } from "@/i18n";

const props = defineProps<{ path: string; disabled?: boolean }>();
const emit = defineEmits<{ navigate: [path: string] }>();
const editing = ref(false);
const draft = ref("");
const input = ref<InputInst>();
const editor = ref<HTMLElement>();
const overflowOpen = ref(false);
const viewport = ref<HTMLElement>();
const measurement = ref<HTMLElement>();
const availableWidth = ref(0);
const widths = ref<number[]>([]);
const overflowWidth = ref(36);
const segments = computed(() => {
  if (!props.path.startsWith("/"))
    return [{ label: props.path || ".", path: props.path || "." }];
  const names = props.path.split("/").filter(Boolean);
  return [
    { label: "/", path: "/" },
    ...names.map((label, index) => ({
      label,
      path: `/${names.slice(0, index + 1).join("/")}`,
    })),
  ];
});
const firstVisible = computed(() => {
  const sizes = widths.value;
  if (!sizes.length || !availableWidth.value) return 0;
  let total = sizes.reduce((sum, width) => sum + width, 0);
  if (total <= availableWidth.value) return 0;
  total += overflowWidth.value;
  let start = 0;
  while (start < segments.value.length - 1 && total > availableWidth.value) {
    total -= sizes[start] ?? 0;
    start++;
  }
  return start;
});
const visible = computed(() => segments.value.slice(firstVisible.value));
const hidden = computed(() =>
  segments.value.slice(0, firstVisible.value).map((segment) => ({
    key: segment.path,
    label: segment.path,
  })),
);
function measure() {
  if (!viewport.value || !measurement.value) return;
  availableWidth.value = Math.max(0, viewport.value.clientWidth - 28);
  widths.value = Array.from(
    measurement.value.querySelectorAll<HTMLElement>("[data-segment]"),
    (node) => node.getBoundingClientRect().width,
  );
  overflowWidth.value =
    measurement.value
      .querySelector<HTMLElement>("[data-overflow]")
      ?.getBoundingClientRect().width ?? 36;
}
let observer: ResizeObserver | undefined;
onMounted(() => {
  observer = new ResizeObserver(measure);
  if (viewport.value) observer.observe(viewport.value);
  if (measurement.value) observer.observe(measurement.value);
  measure();
});
onBeforeUnmount(() => observer?.disconnect());
watch(
  () => props.path,
  () => {
    editing.value = false;
    void nextTick(measure);
  },
);
watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) editing.value = false;
  },
);
async function edit() {
  if (props.disabled) return;
  draft.value = props.path;
  editing.value = true;
  await nextTick();
  input.value?.focus();
  input.value?.select();
}
function navigate(path: string) {
  if (props.disabled || !path.trim()) return;
  overflowOpen.value = false;
  editing.value = false;
  emit("navigate", path);
  void nextTick(measure);
}
function finishEditing() {
  editing.value = false;
  void nextTick(measure);
}
function leaveEditor(event: FocusEvent) {
  if (
    event.relatedTarget instanceof Node &&
    editor.value?.contains(event.relatedTarget)
  )
    return;
  finishEditing();
}
</script>

<template>
  <div class="sftp-path-bar" :class="{ disabled }">
    <div
      v-if="editing"
      ref="editor"
      class="path-editor"
      @focusout="leaveEditor"
      @keydown.esc.stop.prevent="finishEditing"
    >
      <NInput
        ref="input"
        v-model:value="draft"
        :disabled="disabled"
        :bordered="false"
        :theme-overrides="{
          color: 'transparent',
          colorFocus: 'transparent',
          boxShadowFocus: 'none',
        }"
        :placeholder="t('files.path.placeholder')"
        :aria-label="t('files.path.label')"
        @keydown.enter.prevent="navigate(draft)"
      />
      <NButton
        quaternary
        :disabled="disabled || !draft.trim()"
        :aria-label="t('files.actions.go')"
        :title="t('files.actions.go')"
        @mousedown.prevent
        @click="navigate(draft)"
      >
        <NIcon :size="17" aria-hidden="true"><IconArrowRight /></NIcon>
      </NButton>
    </div>
    <nav
      v-show="!editing"
      ref="viewport"
      class="breadcrumbs"
      :aria-label="t('files.path.label')"
      @click.self="edit"
    >
      <NDropdown
        v-if="hidden.length"
        trigger="hover"
        :show="overflowOpen"
        :options="hidden"
        :disabled="disabled"
        @update:show="(show) => (overflowOpen = show)"
        @select="(path: string) => navigate(path)"
      >
        <button
          type="button"
          class="overflow-button"
          :disabled="disabled"
          :aria-label="t('files.navigation.collapsed')"
          :aria-expanded="overflowOpen"
          aria-haspopup="menu"
          @click.stop="overflowOpen = !overflowOpen"
          @keydown.down.prevent="overflowOpen = true"
          @keydown.esc.stop.prevent="overflowOpen = false"
        >
          <NIcon :size="17" aria-hidden="true"><IconDots /></NIcon>
        </button>
      </NDropdown>
      <span
        v-for="(segment, index) in visible"
        :key="segment.path"
        class="path-segment"
        :class="{ current: index === visible.length - 1 }"
      >
        <NIcon
          v-if="index > 0 || hidden.length"
          class="path-separator"
          :size="13"
          aria-hidden="true"
        >
          <IconChevronRight />
        </NIcon>
        <button
          type="button"
          class="segment-button"
          :disabled="disabled"
          :title="segment.path"
          :aria-current="index === visible.length - 1 ? 'location' : undefined"
          @click.stop="navigate(segment.path)"
        >
          {{ segment.label }}
        </button>
      </span>
      <button
        type="button"
        class="edit-path"
        :disabled="disabled"
        :aria-label="t('files.navigation.edit')"
        :title="t('files.navigation.edit')"
        @click.stop="edit"
      />
    </nav>
    <div ref="measurement" class="path-measurement" aria-hidden="true" inert>
      <span data-overflow class="overflow-button"><IconDots :size="17" /></span>
      <span
        v-for="(segment, index) in segments"
        :key="segment.path"
        data-segment
        class="path-segment"
      >
        <IconChevronRight v-if="index > 0" class="path-separator" :size="13" />
        <span class="segment-button">{{ segment.label }}</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.sftp-path-bar {
  position: relative;
  flex: 1 1 0;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
}
.sftp-path-bar:focus-within {
  border-color: var(--accent);
}
.breadcrumbs,
.path-editor {
  display: flex;
  align-items: center;
  min-width: 0;
  height: 34px;
}
.breadcrumbs {
  padding-inline: 5px;
  overflow: hidden;
  cursor: text;
}
.path-editor :deep(.n-input) {
  flex: 1;
  min-width: 0;
  background: transparent;
}
.path-editor :deep(.n-button) {
  flex: none;
  width: 32px;
  height: 32px;
  padding: 0;
}
.path-segment {
  display: inline-flex;
  align-items: center;
  flex: none;
  min-width: 0;
}
.path-segment.current {
  flex: 0 1 auto;
}
.segment-button,
.overflow-button,
.edit-path {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 28px;
  border: 0;
  border-radius: 4px;
  color: inherit;
  background: transparent;
  font: inherit;
  cursor: pointer;
}
.segment-button {
  display: block;
  padding: 0 7px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.overflow-button {
  flex: none;
  width: 28px;
}
.segment-button:hover,
.overflow-button:hover {
  background: var(--surface-hover);
}
.segment-button:focus-visible,
.overflow-button:focus-visible,
.edit-path:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.path-separator {
  flex: none;
  color: var(--muted);
}
.edit-path {
  flex: 1 0 24px;
  padding: 0;
  cursor: text;
}
.path-measurement {
  position: absolute;
  display: flex;
  width: max-content;
  visibility: hidden;
  pointer-events: none;
  inset: 0 auto auto 0;
}
.disabled {
  opacity: 0.55;
}
.disabled button {
  cursor: default;
}
</style>

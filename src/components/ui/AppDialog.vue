<script setup lang="ts">
import { NModal, NScrollbar } from "naive-ui";
defineProps<{
  open: boolean;
  title: string;
  description?: string;
  wide?: boolean;
  locked?: boolean;
  embedded?: boolean;
}>();
const emit = defineEmits<{ "update:open": [value: boolean] }>();
</script>
<template>
  <section v-if="embedded" class="embedded-dialog">
    <NScrollbar class="embedded-dialog-body" :aria-label="`${title}内容`">
      <slot />
    </NScrollbar>
    <footer v-if="$slots.footer" class="embedded-dialog-footer">
      <slot name="footer" />
    </footer>
  </section>
  <NModal
    v-else
    :show="open"
    preset="card"
    content-scrollable
    :title="title"
    :closable="!locked"
    :close-on-esc="!locked"
    :mask-closable="false"
    :auto-focus="true"
    :trap-focus="true"
    :style="{
      maxHeight: 'calc(100dvh - 40px)',
      width: wide
        ? 'min(1050px, calc(100vw - 40px))'
        : 'min(550px, calc(100vw - 40px))',
    }"
    :data-modal-open="open ? '' : undefined"
    aria-modal="true"
    role="dialog"
    :aria-label="title"
    @update:show="(value) => !locked && emit('update:open', value)"
  >
    <p v-if="description" class="muted">{{ description }}</p>
    <slot />
    <template v-if="$slots.footer" #footer>
      <slot name="footer" />
    </template>
  </NModal>
</template>

<style scoped>
.embedded-dialog {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.embedded-dialog-body {
  flex: 1;
  min-height: 0;
}
.embedded-dialog-footer {
  flex-shrink: 0;
  padding: 16px 28px;
  border-top: 1px solid var(--border);
  background: var(--surface);
}
</style>

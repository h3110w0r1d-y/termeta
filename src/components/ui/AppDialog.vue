<script setup lang="ts">
import {
  DialogRoot,
  DialogPortal,
  DialogOverlay,
  DialogContent,
  DialogTitle,
  DialogDescription,
  DialogClose,
} from "reka-ui";
defineProps<{
  open: boolean;
  title: string;
  description?: string;
  wide?: boolean;
  locked?: boolean;
}>();
const emit = defineEmits<{ "update:open": [value: boolean] }>();
</script>
<template>
  <DialogRoot
    :open="open"
    @update:open="(value) => !locked && emit('update:open', value)"
  >
    <DialogPortal
      ><DialogOverlay class="dialog-overlay" /><DialogContent
        data-modal-open
        class="dialog-panel"
        :class="{ wide }"
        @escape-key-down="(event) => locked && event.preventDefault()"
        @pointer-down-outside="(event) => event.preventDefault()"
      >
        <header class="dialog-header">
          <div>
            <DialogTitle class="dialog-title">{{ title }}</DialogTitle
            ><DialogDescription class="muted">{{
              description || " "
            }}</DialogDescription>
          </div>
          <DialogClose v-if="!locked" class="icon-button" aria-label="关闭"
            >✕</DialogClose
          >
        </header>
        <slot /> </DialogContent
    ></DialogPortal>
  </DialogRoot>
</template>

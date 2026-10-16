<script setup lang="ts">
import { NIcon } from "naive-ui";
import type { VaultSection } from "../modules/workspace/store";
defineProps<{ section: VaultSection }>();
</script>
<template>
  <NIcon :size="18" aria-hidden="true">
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      focusable="false"
    >
      <template v-if="section === 'hosts'">
        <rect x="3" y="3" width="18" height="13" rx="2" />
        <path d="M8 21h8m-4-5v5M7 7l3 3-3 3m6 0h4" />
      </template>
      <template v-else-if="section === 'credentials'">
        <circle cx="8" cy="8" r="5" />
        <path d="m12 12 9 9m-3-3 3-3m-6 0 3-3M6.5 6.5h.01" />
      </template>
      <template v-else-if="section === 'known-hosts'">
        <path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3Z" />
        <path d="m8 12 3 3 5-6" />
      </template>
      <template v-else>
        <path d="M3 11a9 9 0 1 1 2.6 7.4M3 4v7h7m2-4v5l3 2" />
      </template>
    </svg>
  </NIcon>
</template>

<script setup lang="ts">
import { t } from "../../i18n";
import { NIcon } from "naive-ui";
import type { FileEntry } from "../../services/desktop/types";
defineProps<{
  kind: FileEntry["kind"];
  targetKind?: FileEntry["targetKind"];
}>();
</script>
<template>
  <NIcon
    :size="16"
    :aria-label="
      {
        directory: t('文件夹'),
        file: t('文件'),
        symlink:
          targetKind === 'directory'
            ? t('文件夹软连接')
            : targetKind === 'file'
              ? t('文件软连接')
              : t('软连接（目标类型未知）'),
        other: t('其他文件'),
      }[kind]
    "
    role="img"
  >
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path
        v-if="
          kind === 'directory' ||
          (kind === 'symlink' && targetKind === 'directory')
        "
        d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v10H3Z"
      />
      <path
        v-else-if="kind === 'symlink'"
        d="m10 14 4-4m-6 6-1 1a3.5 3.5 0 0 1-5-5l4-4a3.5 3.5 0 0 1 5 0m2 8a3.5 3.5 0 0 0 5 0l4-4a3.5 3.5 0 0 0-5-5l-1 1"
      />
      <path v-else d="M14 3H5v18h14V8Zm0 0v5h5M8 13h8M8 17h6" />
      <path
        v-if="kind === 'symlink' && targetKind === 'directory'"
        d="M9 13h7m-3-3 3 3-3 3"
      />
    </svg>
  </NIcon>
</template>

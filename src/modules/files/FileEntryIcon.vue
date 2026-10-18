<script setup lang="ts">
import { computed } from "vue";
import { t } from "../../i18n";
import { NIcon } from "naive-ui";
import {
  IconFile,
  IconFileSymlink,
  IconFileText,
  IconFolder,
  IconFolderSymlink,
  IconLink,
} from "@tabler/icons-vue";
import type { FileEntry } from "../../services/desktop/types";

const props = defineProps<{
  kind: FileEntry["kind"];
  targetKind?: FileEntry["targetKind"];
}>();
const icon = computed(() => {
  if (props.kind === "directory") return IconFolder;
  if (props.kind === "file") return IconFileText;
  if (props.kind === "other") return IconFile;
  if (props.targetKind === "directory") return IconFolderSymlink;
  if (props.targetKind === "file") return IconFileSymlink;
  return IconLink;
});
</script>

<template>
  <NIcon
    :size="16"
    :aria-label="
      {
        directory: t('files.types.directory'),
        file: t('files.types.file'),
        symlink:
          targetKind === 'directory'
            ? t('files.types.directoryLink')
            : targetKind === 'file'
              ? t('files.types.fileLink')
              : t('files.types.unknownLink'),
        other: t('files.types.other'),
      }[kind]
    "
    role="img"
  >
    <component :is="icon" aria-hidden="true" />
  </NIcon>
</template>

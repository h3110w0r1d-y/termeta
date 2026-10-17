<script setup lang="ts">
import { t } from "../../i18n";
import { NFlex } from "naive-ui";
import AppButton from "../../components/ui/AppButton.vue";
import AppDialog from "../../components/ui/AppDialog.vue";
import { pendingPaste, resolvePaste } from "./paste";
</script>
<template>
  <AppDialog
    :open="!!pendingPaste"
    :title="t('terminal.paste.confirmMultiline')"
    :description="t('terminal.paste.warning')"
    @update:open="(value) => !value && resolvePaste(false)"
  >
    <template v-if="pendingPaste">
      <p>
        {{
          pendingPaste.targetCount > 1
            ? t("terminal.paste.broadcastDescription", {
                lines: pendingPaste.lines,
                count: pendingPaste.targetCount,
              })
            : t("terminal.paste.currentDescription", {
                lines: pendingPaste.lines,
              })
        }}
      </p>
      <pre class="paste-preview">{{ pendingPaste.text }}</pre>
    </template>
    <template #footer>
      <NFlex justify="end">
        <AppButton @click="resolvePaste(false)">
          {{ t("common.actions.cancel") }}
        </AppButton>
        <AppButton class="primary" @click="resolvePaste(true)">
          {{
            pendingPaste && pendingPaste.targetCount > 1
              ? t("terminal.paste.confirmBroadcast", {
                  targetCount: pendingPaste.targetCount,
                })
              : t("terminal.paste.confirmCurrent")
          }}
        </AppButton>
      </NFlex>
    </template>
  </AppDialog>
</template>

<style scoped>
.paste-preview {
  background: var(--input);
  max-height: 40dvh;
  overflow: auto;
  padding: 14px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font: 12px/1.6 monospace;
}
</style>

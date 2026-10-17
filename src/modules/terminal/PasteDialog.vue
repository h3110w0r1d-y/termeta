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
    :title="t('确认多行粘贴')"
    :description="t('换行可能立即执行远端命令。请核对全部内容后继续。')"
    @update:open="(value) => !value && resolvePaste(false)"
  >
    <template v-if="pendingPaste">
      <p>
        {{
          pendingPaste.targetCount > 1
            ? t("将粘贴 {lines} 行到 {count} 个终端", {
                lines: pendingPaste.lines,
                count: pendingPaste.targetCount,
              })
            : t("将粘贴 {lines} 行到当前终端", { lines: pendingPaste.lines })
        }}
      </p>
      <pre class="paste-preview">{{ pendingPaste.text }}</pre>
    </template>
    <template #footer>
      <NFlex justify="end">
        <AppButton @click="resolvePaste(false)">{{ t("取消") }}</AppButton>
        <AppButton class="primary" @click="resolvePaste(true)">
          {{
            pendingPaste && pendingPaste.targetCount > 1
              ? t("确认广播到 {targetCount} 个终端", {
                  targetCount: pendingPaste.targetCount,
                })
              : t("确认粘贴到当前连接")
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

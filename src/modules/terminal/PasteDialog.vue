<script setup lang="ts">
import { NFlex } from "naive-ui";
import AppButton from "../../components/ui/AppButton.vue";
import AppDialog from "../../components/ui/AppDialog.vue";
import { pendingPaste, resolvePaste } from "./paste";
</script>
<template>
  <AppDialog
    :open="!!pendingPaste"
    title="确认多行粘贴"
    description="换行可能立即执行远端命令。请核对全部内容后继续。"
    @update:open="(value) => !value && resolvePaste(false)"
    ><template v-if="pendingPaste"
      ><p>即将粘贴 {{ pendingPaste.lines }} 行；控制字符以原始文本显示。</p>
      <pre class="paste-preview">{{ pendingPaste.text }}</pre>
    </template>
    <template #footer>
      <NFlex justify="end">
        <AppButton @click="resolvePaste(false)">取消</AppButton
        ><AppButton class="primary" @click="resolvePaste(true)">
          确认粘贴到当前连接
        </AppButton>
      </NFlex></template
    ></AppDialog
  >
</template>

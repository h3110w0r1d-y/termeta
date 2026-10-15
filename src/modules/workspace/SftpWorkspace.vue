<script setup lang="ts">
import {
  NAlert,
  NCard,
  NEmpty,
  NGrid,
  NGridItem,
  NSpace,
  NTag,
} from "naive-ui";
</script>
<template>
  <section
    id="page-sftp"
    aria-labelledby="workspace-tab-sftp"
    class="sftp-workspace"
    role="tabpanel"
    aria-label="SFTP"
  >
    <NSpace vertical :size="24">
      <header class="page-heading">
        <div>
          <h1>SFTP</h1>
          <p class="muted">双栏文件工作区</p>
        </div>
        <NTag :bordered="false">尚未开放</NTag>
      </header>
      <NAlert type="info" :show-icon="true">
        本期提供 SSH 终端；双栏文件浏览与传输将在后续版本接入。
      </NAlert>
      <NGrid :cols="2" :x-gap="16">
        <NGridItem v-for="side in ['左侧', '右侧']" :key="side">
          <NCard :title="side + '文件浏览器'" size="small">
            <template #header-extra><NTag size="small">未连接</NTag></template>
            <NEmpty
              class="file-browser-placeholder"
              description="后续可选择本地目录或远程主机"
            />
          </NCard>
        </NGridItem>
      </NGrid>
      <p class="muted">此页面不会自动建立 SSH 连接。</p>
    </NSpace>
  </section>
</template>
<style scoped>
.file-browser-placeholder {
  padding: 80px 12px;
}
</style>

<script setup lang="ts">
import { NMenu } from "naive-ui";
import AppScrollArea from "../components/ui/AppScrollArea.vue";
import HostsPage from "../modules/hosts/HostsPage.vue";
import CredentialsPage from "../modules/credentials/CredentialsPage.vue";
import SettingsPage from "../modules/settings/SettingsPage.vue";
import { useWorkspace } from "../modules/workspace/store";
import type { HostProfile } from "../services/desktop/types";
const workspace = useWorkspace();
const emit = defineEmits<{
  connect: [host: HostProfile];
  local: [];
  edit: [host?: HostProfile, copy?: boolean, groupId?: string | null];
}>();
</script>
<template>
  <section
    id="page-home"
    aria-labelledby="workspace-tab-home"
    v-show="workspace.activeId === 'home'"
    class="home-workspace"
    role="tabpanel"
    aria-label="首页"
  >
    <aside class="home-navigation">
      <NMenu
        :value="workspace.homeSection"
        :options="[
          { key: 'hosts', label: '主机' },
          { key: 'credentials', label: '凭据' },
          { key: 'settings', label: '设置' },
        ]"
        @update:value="workspace.openHome($event)"
      />
    </aside>
    <div class="home-content">
      <!-- 显隐挂在原生容器上，避免滚动组件内部的观察器根节点丢失指令。 -->
      <div v-show="workspace.homeSection === 'hosts'" class="home-page-scroll">
        <HostsPage
          :active="
            workspace.activeId === 'home' && workspace.homeSection === 'hosts'
          "
          @connect="emit('connect', $event)"
          @local="emit('local')"
          @edit="(host, copy, groupId) => emit('edit', host, copy, groupId)"
        />
      </div>
      <AppScrollArea
        v-if="workspace.homeSection === 'credentials'"
        class="home-page-scroll"
        label="凭据内容"
      >
        <CredentialsPage />
      </AppScrollArea>
      <SettingsPage
        v-if="workspace.homeSection === 'settings'"
        :embedded="true"
        :open="true"
      />
    </div>
  </section>
</template>
<style scoped>
.home-workspace {
  display: flex;
  flex: 1;
  min-height: 0;
}
.home-navigation {
  width: 190px;
  flex-shrink: 0;
  padding: 12px;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border);
  background: var(--surface);
}
.home-content {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.home-page-scroll {
  flex: 1;
  min-height: 0;
}
.tabs-left .home-navigation {
  width: 160px;
  padding-left: 8px;
  padding-right: 8px;
}
@media (max-width: 900px) {
  .home-navigation {
    width: 150px;
    padding-left: 8px;
    padding-right: 8px;
  }
  .tabs-left .home-navigation {
    width: 130px;
  }
}
</style>

<script setup lang="ts">
import { NMenu } from "naive-ui";
import { h } from "vue";
import VaultNavigationIcon from "./VaultNavigationIcon.vue";
import AppScrollArea from "../components/ui/AppScrollArea.vue";
import HostsPage from "../modules/hosts/HostsPage.vue";
import CredentialsPage from "../modules/credentials/CredentialsPage.vue";
import KnownHostsPage from "../modules/hosts/KnownHostsPage.vue";
import ConnectionHistoryPage from "../modules/history/ConnectionHistoryPage.vue";
import { useWorkspace, type VaultSection } from "../modules/workspace/store";
import type { HostProfile } from "../services/desktop/types";
const workspace = useWorkspace();
const navigationItems: { key: VaultSection; label: string }[] = [
  { key: "hosts", label: "主机" },
  { key: "credentials", label: "凭据" },
  { key: "known-hosts", label: "主机信任" },
  { key: "history", label: "连接历史" },
];
const navigationOptions = navigationItems.map((item) => ({
  ...item,
  icon: () => h(VaultNavigationIcon, { section: item.key }),
}));
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
    class="vault-workspace"
    role="tabpanel"
    aria-label="保险箱"
  >
    <aside class="vault-navigation">
      <NMenu
        :value="workspace.vaultSection"
        :options="navigationOptions"
        @update:value="workspace.openVault($event)"
      />
    </aside>
    <div class="vault-content">
      <!-- 显隐挂在原生容器上，避免滚动组件内部的观察器根节点丢失指令。 -->
      <div
        v-show="workspace.vaultSection === 'hosts'"
        class="vault-page-scroll"
      >
        <HostsPage
          :active="
            workspace.activeId === 'home' && workspace.vaultSection === 'hosts'
          "
          @connect="emit('connect', $event)"
          @local="emit('local')"
          @edit="(host, copy, groupId) => emit('edit', host, copy, groupId)"
        />
      </div>
      <AppScrollArea
        v-if="workspace.vaultSection === 'credentials'"
        class="vault-page-scroll"
        label="凭据内容"
      >
        <CredentialsPage />
      </AppScrollArea>
      <AppScrollArea
        v-if="workspace.vaultSection === 'known-hosts'"
        class="vault-page-scroll"
        label="主机信任内容"
      >
        <KnownHostsPage />
      </AppScrollArea>
      <AppScrollArea
        v-if="workspace.vaultSection === 'history'"
        class="vault-page-scroll"
        label="连接历史内容"
      >
        <ConnectionHistoryPage />
      </AppScrollArea>
    </div>
  </section>
</template>
<style scoped>
.vault-workspace {
  display: flex;
  flex: 1;
  min-height: 0;
}
.vault-navigation {
  width: 190px;
  flex-shrink: 0;
  padding: 12px;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border);
  background: var(--surface);
}
.vault-content {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.vault-page-scroll {
  flex: 1;
  min-height: 0;
}
.tabs-left .vault-navigation {
  width: 160px;
  padding-left: 8px;
  padding-right: 8px;
}
@media (max-width: 900px) {
  .vault-navigation {
    width: 150px;
    padding-left: 8px;
    padding-right: 8px;
  }
  .tabs-left .vault-navigation {
    width: 130px;
  }
}
</style>

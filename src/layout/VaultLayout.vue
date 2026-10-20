<script setup lang="ts">
import { t } from "../i18n";
import { NMenu, NButton } from "naive-ui";
import { IconSettings } from "@tabler/icons-vue";
import { commands } from "../services/commands/registry";
import { computed, h } from "vue";
import VaultNavigationIcon from "./VaultNavigationIcon.vue";
import AppScrollArea from "../components/ui/AppScrollArea.vue";
import HostsPage from "../modules/hosts/HostsPage.vue";
import PortForwardingPage from "../modules/forwarding/PortForwardingPage.vue";
import CredentialsPage from "../modules/credentials/CredentialsPage.vue";
import KnownHostsPage from "../modules/hosts/KnownHostsPage.vue";
import SessionLogsPage from "../modules/session-logs/SessionLogsPage.vue";
import SnippetsPage from "../modules/snippets/SnippetsPage.vue";
import {
  useWorkbenchStore,
  type VaultSection,
  type TerminalOpenMode,
} from "../modules/workbench/store";
import type { HostKind, HostProfile } from "../services/desktop/types";
const workbench = useWorkbenchStore();
const navigationOptions = computed(() => {
  const items: { key: VaultSection; label: string }[] = [
    { key: "hosts", label: t("common.navigation.hosts") },
    { key: "forwarding", label: t("common.navigation.forwarding") },
    { key: "credentials", label: t("common.navigation.credentials") },
    { key: "snippets", label: t("common.navigation.snippets") },
    { key: "known-hosts", label: t("common.navigation.knownHosts") },
    { key: "logs", label: t("common.navigation.logs") },
  ];
  return items.map((item) => ({
    ...item,
    icon: () => h(VaultNavigationIcon, { section: item.key }),
  }));
});
const emit = defineEmits<{
  connect: [host: HostProfile];
  connectMany: [HostProfile[], TerminalOpenMode];
  local: [];
  create: [HostKind, string | null];
  quick: [HostKind];
  edit: [host?: HostProfile, copy?: boolean, groupId?: string | null];
}>();
</script>
<template>
  <section
    id="page-home"
    aria-labelledby="app-tab-home"
    v-show="workbench.activeId === 'home'"
    class="vault-page"
    role="tabpanel"
    :aria-label="t('common.navigation.vault')"
  >
    <aside class="vault-navigation">
      <NMenu
        :root-indent="18"
        :value="workbench.vaultSection"
        :options="navigationOptions"
        @update:value="workbench.openVault($event)"
      />
      <div class="vault-settings-entry">
        <NButton block quaternary @click="commands.execute('settings.open')">
          <template #icon>
            <IconSettings :size="18" aria-hidden="true" />
          </template>
          {{ t("settings.title") }}
        </NButton>
      </div>
    </aside>
    <div class="vault-content">
      <!-- 显隐挂在原生容器上，避免滚动组件内部的观察器根节点丢失指令。 -->
      <div
        v-show="workbench.vaultSection === 'hosts'"
        class="vault-page-scroll"
      >
        <HostsPage
          :active="
            workbench.activeId === 'home' && workbench.vaultSection === 'hosts'
          "
          @connect="emit('connect', $event)"
          @connect-many="
            (profiles, mode) => emit('connectMany', profiles, mode)
          "
          @local="emit('local')"
          @create="(kind, groupId) => emit('create', kind, groupId)"
          @quick="emit('quick', $event)"
          @edit="(host, copy, groupId) => emit('edit', host, copy, groupId)"
        />
      </div>
      <div
        v-show="workbench.vaultSection === 'forwarding'"
        class="vault-page-scroll"
      >
        <AppScrollArea :label="t('common.regions.forwarding')">
          <PortForwardingPage />
        </AppScrollArea>
      </div>
      <AppScrollArea
        v-if="workbench.vaultSection === 'credentials'"
        class="vault-page-scroll"
        :label="t('common.regions.credentials')"
      >
        <CredentialsPage />
      </AppScrollArea>
      <AppScrollArea
        v-if="workbench.vaultSection === 'snippets'"
        class="vault-page-scroll"
        :label="t('common.regions.snippets')"
      >
        <SnippetsPage />
      </AppScrollArea>
      <AppScrollArea
        v-if="workbench.vaultSection === 'known-hosts'"
        class="vault-page-scroll"
        :label="t('common.regions.knownHosts')"
      >
        <KnownHostsPage />
      </AppScrollArea>
      <AppScrollArea
        v-if="workbench.vaultSection === 'logs'"
        class="vault-page-scroll"
        :label="t('common.regions.logs')"
      >
        <SessionLogsPage />
      </AppScrollArea>
    </div>
  </section>
</template>
<style scoped>
.vault-page {
  display: flex;
  flex: 1;
  min-height: 0;
}
.vault-navigation {
  width: 190px;
  flex-shrink: 0;
  padding-top: 3px;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border);
  background: var(--surface);
}
.vault-settings-entry {
  margin-top: auto;
  padding: 12px 10px;
  border-top: 1px solid var(--border);
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
</style>

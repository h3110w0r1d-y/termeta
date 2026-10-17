<script setup lang="ts">
import { t } from "./i18n";
import { naiveLocale, naiveDateLocale } from "./i18n";
import { CloseOutline } from "@vicons/ionicons5";
import { closeContextMenuOnOutside } from "./components/ui/contextMenu";
import {
  NIcon,
  NConfigProvider,
  NGlobalStyle,
  NAlert,
  NDropdown,
} from "naive-ui";
import { useAppAppearance } from "./layout/useAppAppearance";
import { useNativeMenuLocale } from "./layout/useNativeMenuLocale";
import VaultLayout from "./layout/VaultLayout.vue";
import VaultGate from "./components/VaultGate.vue";
import WindowControls from "./components/WindowControls.vue";
import { isMac } from "./services/commands/registry";
import AppButton from "./components/ui/AppButton.vue";
import AppInput from "./components/ui/AppInput.vue";
import AppDialog from "./components/ui/AppDialog.vue";
import { confirmAction } from "./components/ui/confirmation";
import {
  computed,
  nextTick,
  onMounted,
  onBeforeUnmount,
  ref,
  watch,
} from "vue";
import ConfirmationDialog from "./components/ui/ConfirmationDialog.vue";
import AppTabs from "./modules/workbench/AppTabs.vue";
import SftpBrowser from "./modules/files/SftpBrowser.vue";
import SessionLogStage from "./modules/session-logs/SessionLogStage.vue";
import NewTabDialog from "./modules/hosts/NewTabDialog.vue";
import SaveWorkspaceDialog from "./modules/workbench/SaveWorkspaceDialog.vue";
import HostEditor from "./modules/hosts/HostEditor.vue";
import SerialConnectDialog from "./modules/terminal/SerialConnectDialog.vue";
import TerminalStage from "./modules/workbench/TerminalStage.vue";
import { useTerminalMenu } from "./modules/workbench/useTerminalMenu";
import { useTerminalClose } from "./modules/workbench/useTerminalClose";
import PasteDialog from "./modules/terminal/PasteDialog.vue";
import ChallengeDialog from "./modules/terminal/ChallengeDialog.vue";
import { useHosts } from "./modules/hosts/store";
import { usePortForwards } from "./modules/forwarding/store";
import { useSettings } from "./modules/settings/store";
import {
  useWorkbenchStore,
  stateLabels,
  type TerminalOpenMode,
} from "./modules/workbench/store";
import {
  schemeForTerminal,
  terminalRegistry,
} from "./modules/terminal/registry";
import { needsCloseConfirmation } from "./modules/terminal/activity";
import { commands } from "./services/commands/registry";
import { desktop, errorMessage } from "./services/desktop/client";
import type {
  HostKind,
  HostProfile,
  SerialConfig,
  OpenTarget,
} from "./services/desktop/types";
import { useSettingsSync } from "./modules/settings/useSettingsSync";
import { useBackupRestored } from "./modules/backup/useBackupRestored";
const settingsSync = useSettingsSync();
const nativeMenu = useNativeMenuLocale();
const hosts = useHosts(),
  settings = useSettings(),
  workbench = useWorkbenchStore();
const portForwards = usePortForwards();
const newTabOpen = ref(false);
const serialOpen = ref(false);
const serialSave = ref(false);
const editorKind = ref<"ssh" | "telnet">("ssh");
const editorOpen = ref(false),
  quick = ref(false),
  editing = ref<HostProfile>(),
  duplicate = ref(false),
  newHostGroup = ref<string | null>(null),
  initError = ref(""),
  findOpen = ref(false),
  query = ref("");
watch(settingsSync.error, (message) => {
  if (message) initError.value = message;
});
const vaultReady = ref(false);
const savingLogs = ref(false);
useBackupRestored(
  async () => {
    if (vaultReady.value)
      await Promise.all([hosts.load(), portForwards.load()]);
  },
  (cause) => {
    initError.value = errorMessage(cause);
  },
);
async function loadUnlocked() {
  await Promise.all([hosts.load(), portForwards.load(), settingsSync.start()]);
  vaultReady.value = true;
}
const connected = computed(
  () => workbench.terminals.filter((t) => t.state === "connected").length,
);
const terminalBackground = computed(() =>
  workbench.activeTerminal
    ? schemeForTerminal(workbench.activeTerminal).colors.background
    : undefined,
);
const { naiveTheme, naiveThemeOverrides } = useAppAppearance();
let heartbeat: ReturnType<typeof setInterval> | undefined,
  unlisten: (() => void) | undefined,
  unlistenExit: (() => void) | undefined,
  closingWindow = false,
  preparingClose = false,
  exitingApp = false;
function edit(host?: HostProfile, copy = false, groupId: string | null = null) {
  if (host?.kind === "workspace") return;
  newHostGroup.value = groupId;
  editing.value = host;
  duplicate.value = copy;
  quick.value = false;
  if (host?.kind === "serial") {
    serialSave.value = true;
    serialOpen.value = true;
  } else {
    editorKind.value = host?.kind === "telnet" ? "telnet" : "ssh";
    editorOpen.value = true;
  }
}
function createHost(kind: HostKind, groupId: string | null) {
  if (kind !== "serial") {
    edit(undefined, false, groupId);
    editorKind.value = kind;
  } else {
    editing.value = undefined;
    duplicate.value = false;
    newHostGroup.value = groupId;
    serialSave.value = true;
    serialOpen.value = true;
  }
}
const savingGroupId = ref("");
function openNewTab() {
  if (savingLogs.value) return;
  newTabOpen.value = true;
}
function quickConnect(kind: HostKind = "ssh") {
  if (savingLogs.value) return;
  newHostGroup.value = null;
  editing.value = undefined;
  quick.value = true;
  duplicate.value = false;
  if (kind === "serial") {
    serialSave.value = false;
    serialOpen.value = true;
  } else {
    editorKind.value = kind;
    editorOpen.value = true;
  }
}
function openLocalTerminal() {
  if (savingLogs.value) return;
  workbench.add("本地终端", { source: "local" });
}
function openSerial(config: SerialConfig) {
  if (savingLogs.value) return;
  workbench.add(config.path, { source: "serial", config });
}
function connect(host: HostProfile) {
  connectMany([host], "tabs");
}
function connectMany(profiles: HostProfile[], mode: TerminalOpenMode) {
  if (savingLogs.value) return;
  try {
    workbench.addSavedHosts(profiles, mode);
  } catch (e) {
    initError.value = errorMessage(e);
  }
}
function ephemeral(target: OpenTarget, title: string) {
  if (savingLogs.value) return;
  workbench.add(title, target);
}
const { closeTab, closePane } = useTerminalClose();
function perform(action: () => void | Promise<void>) {
  try {
    Promise.resolve(action()).catch((e) => {
      if (workbench.activeTerminal)
        workbench.activeTerminal.error = errorMessage(e);
      else initError.value = errorMessage(e);
    });
  } catch (e) {
    initError.value = errorMessage(e);
  }
}
const {
  tabMenu,
  terminalMenuProps,
  tabMenuOptions,
  openTabMenu,
  openTerminalMenu,
  doubleClickTab,
  selectTabAction,
} = useTerminalMenu({
  perform,
  closeTab,
  saveWorkspace: (id) => {
    savingGroupId.value = id;
  },
  focusTerminal,
  openFind: () => {
    findOpen.value = true;
  },
});
async function focusTerminal(id: string) {
  const group = workbench.groups.find((item) => item.id === id);
  if (!group) return;
  await nextTick();
  terminalRegistry.focus(group.focusedTerminalId);
}
function restoreTerminalFocus() {
  // 连接/认证弹窗的焦点陷阱退场后再聚焦，避免库恢复到已经隐藏的保险箱控件。
  if (!findOpen.value && workbench.activeTerminal)
    terminalRegistry.focus(workbench.activeTerminal.terminalId);
}
function closeFind() {
  findOpen.value = false;
  if (workbench.activeTerminal)
    terminalRegistry.focus(workbench.activeTerminal.terminalId);
}
function globalKey(event: KeyboardEvent) {
  if (!vaultReady.value || savingLogs.value) return;
  if ((event.target as HTMLElement).closest(".terminal-host")) return;
  commands.route(event, settings.value.keymap.bindings, false);
}
watch(
  () => settings.value,
  () => terminalRegistry.apply(),
  { deep: true },
);
watch(
  () => [workbench.activeId, workbench.activeTerminal?.terminalId],
  () => {
    tabMenu.value.show = false;
    findOpen.value = false;
    query.value = "";
  },
  { flush: "post" },
);
watch(
  () => workbench.activeTerminal?.schemeId,
  () => terminalRegistry.apply(),
);
async function closeApplication() {
  if (preparingClose || closingWindow) return;
  preparingClose = true;
  try {
    if (
      (workbench.fileTransferring ||
        workbench.terminals.some((terminal) => terminal.sftpTransferring)) &&
      !(await confirmAction(t("window.close.confirmTransfers")))
    )
      return;
    const running = workbench.terminals.filter(needsCloseConfirmation);
    if (
      (running.length > 0 || portForwards.hasActive) &&
      settings.value.confirmClose &&
      !(await confirmAction(
        [
          running.length
            ? t("window.close.runningTerminals", { count: running.length })
            : "",
          portForwards.hasActive ? t("window.close.activeForwarding") : "",
          t("window.close.confirmSessions"),
        ]
          .filter(Boolean)
          .join(" "),
      ))
    )
      return;
    tabMenu.value.show = false;
    savingLogs.value = true;
    // 等待每个日志提交后才删除对应视图；任一失败都阻止窗口/应用退出。
    const results = await Promise.allSettled(
      workbench.terminals.map(async (terminal) => {
        await terminalRegistry.dispose(terminal.terminalId);
        workbench.dismiss(terminal.sessionId ?? "");
        workbench.remove(terminal.terminalId);
      }),
    );
    const failure = results.find((result) => result.status === "rejected");
    if (failure?.status === "rejected") throw failure.reason;
    if (exitingApp) {
      await desktop.finishAppExit();
    } else {
      closingWindow = true;
      await desktop.closeWindow();
    }
  } catch (cause) {
    closingWindow = false;
    initError.value = errorMessage(cause);
  } finally {
    savingLogs.value = false;
    preparingClose = false;
    exitingApp = false;
  }
}
onMounted(async () => {
  commands.register("workbench.newConnection", openNewTab);
  commands.register("workbench.newTab", openNewTab);
  commands.register("workbench.newLocalTerminal", openLocalTerminal);
  commands.register("workbench.closeTab", () => {
    if (workbench.activeLogTab)
      workbench.closeSessionLogTab(workbench.activeLogTab.id);
    else if (workbench.activeGroup) void closeTab(workbench.activeGroup.id);
  });
  commands.register("workbench.nextTab", () => workbench.cycle(1));
  commands.register("workbench.previousTab", () => workbench.cycle(-1));
  commands.register("settings.open", () => perform(desktop.openSettingsWindow));
  commands.register("terminal.copy", () => {
    if (workbench.activeTerminal)
      perform(() =>
        terminalRegistry.copy(workbench.activeTerminal!.terminalId),
      );
  });
  commands.register("terminal.paste", () => {
    if (workbench.activeTerminal)
      perform(() =>
        terminalRegistry.paste(workbench.activeTerminal!.terminalId),
      );
  });
  commands.register("terminal.find", () => (findOpen.value = true));
  commands.register("terminal.scrollToBottom", () => {
    if (workbench.activeTerminal)
      terminalRegistry.bottom(workbench.activeTerminal.terminalId);
  });
  document.addEventListener("keydown", globalKey, true);
  try {
    unlistenExit = await desktop.onAppExitRequested(() => {
      exitingApp = true;
      void closeApplication();
    });
    unlisten = await desktop.onCloseRequested((event) => {
      if (closingWindow) return;
      event.preventDefault();
      void closeApplication();
    });
    heartbeat = setInterval(
      () =>
        vaultReady.value &&
        void desktop.heartbeat().catch((e) => {
          initError.value = errorMessage(e);
        }),
      15000,
    );
  } catch (e) {
    initError.value = errorMessage(e);
  }
});
onBeforeUnmount(() => {
  document.removeEventListener("keydown", globalKey, true);
  if (heartbeat) clearInterval(heartbeat);
  unlisten?.();
  unlistenExit?.();
});
</script>
<template>
  <NConfigProvider
    :theme="naiveTheme"
    :theme-overrides="naiveThemeOverrides"
    :locale="naiveLocale"
    :date-locale="naiveDateLocale"
  >
    <NGlobalStyle />
    <NAlert v-if="nativeMenu.error.value" type="error" role="alert">
      {{ t("common.errors.nativeMenuUpdate") }}
      <AppButton @click="nativeMenu.retry">
        {{ t("common.actions.retry") }}
      </AppButton>
    </NAlert>
    <AppDialog
      :open="savingLogs"
      :locked="true"
      :title="t('window.close.savingLogs')"
    />
    <VaultGate :initialize="loadUnlocked">
      <div
        class="app-shell"
        :inert="savingLogs || undefined"
        :class="{
          'tabs-left': settings.value.tabPosition === 'left',
          'platform-mac': isMac,
        }"
      >
        <header
          v-if="settings.value.tabPosition === 'left'"
          class="window-titlebar"
          data-tauri-drag-region
        >
          <span data-tauri-drag-region>Termeta</span>
          <div class="window-drag-space" data-tauri-drag-region />
          <WindowControls @error="initError = $event" />
        </header>
        <AppTabs
          :tabs="workbench.tabBarItems"
          :active-id="workbench.activeId"
          :position="settings.value.tabPosition"
          :connected="connected"
          @window-error="initError = $event"
          @activate="workbench.activateTab($event)"
          @focus-terminal="focusTerminal"
          @close="closeTab"
          @double-click="doubleClickTab"
          @menu="openTabMenu"
          @create="commands.execute('workbench.newTab')"
        />
        <NDropdown
          :show="tabMenu.show"
          :x="tabMenu.x"
          :y="tabMenu.y"
          :options="tabMenuOptions"
          :menu-props="terminalMenuProps"
          display-directive="show"
          scrollable
          :flip="true"
          trigger="manual"
          placement="bottom-start"
          @select="selectTabAction"
          @clickoutside="closeContextMenuOnOutside($event, tabMenu)"
          @update:show="tabMenu.show = $event"
        />
        <main class="workbench-content">
          <NAlert
            v-if="initError"
            type="error"
            closable
            @close="initError = ''"
          >
            {{ initError }}
          </NAlert>
          <VaultLayout
            @connect="connect"
            @connect-many="connectMany"
            @local="openLocalTerminal"
            @create="createHost"
            @quick="quickConnect"
            @edit="edit"
          />
          <SftpBrowser v-show="workbench.activeId === 'sftp'" />
          <SessionLogStage />
          <!-- 终端宿主始终留在同一组件树内；页面与布局切换仅改变可见性。 -->
          <section
            v-show="!!workbench.activeTerminal"
            class="terminal-area"
            :aria-label="t('terminal.regions.area')"
          >
            <div v-if="findOpen && workbench.activeTerminal" class="find-bar">
              <AppInput
                v-model="query"
                autofocus
                :aria-label="t('terminal.search.label')"
                :placeholder="t('terminal.search.placeholder')"
                @keydown.enter.prevent="
                  terminalRegistry.find(
                    workbench.activeTerminal.terminalId,
                    query,
                    $event.shiftKey,
                  )
                "
                @keydown.esc="closeFind()"
              />
              <AppButton
                @click="
                  terminalRegistry.find(
                    workbench.activeTerminal.terminalId,
                    query,
                    true,
                  )
                "
              >
                {{ t("common.actions.previous") }}
              </AppButton>
              <AppButton
                @click="
                  terminalRegistry.find(
                    workbench.activeTerminal.terminalId,
                    query,
                  )
                "
              >
                {{ t("common.actions.next") }}
              </AppButton>
              <AppButton
                class="icon-button"
                :aria-label="t('terminal.search.close')"
                @click="closeFind()"
              >
                <NIcon :size="16" aria-hidden="true"><CloseOutline /></NIcon>
              </AppButton>
            </div>
            <NAlert v-if="workbench.activeTerminal?.error" type="error">
              {{ workbench.activeTerminal.error }}
            </NAlert>
            <div
              class="terminal-stage"
              :style="{ '--terminal-background': terminalBackground }"
            >
              <TerminalStage @close="closePane" @menu="openTerminalMenu" />
            </div>
            <footer v-if="workbench.activeTerminal" class="status-bar">
              <span>
                {{
                  workbench.activeTerminal.kind === "local"
                    ? t("common.connection.local")
                    : workbench.activeTerminal.kind === "serial"
                      ? t("common.connection.serial")
                      : workbench.activeTerminal.kind === "telnet"
                        ? t("common.connection.telnetUnencrypted")
                        : "SSH"
                }}
                · {{ t(stateLabels[workbench.activeTerminal.state]) }}
              </span>
              <span
                v-if="
                  workbench.activeTerminal.kind === 'serial' &&
                  workbench.activeTerminal.serialConfig
                "
              >
                {{ workbench.activeTerminal.serialConfig.baudRate }} baud ·
                {{ workbench.activeTerminal.serialConfig.charset }}
              </span>
              <span v-else>
                {{ workbench.activeTerminal.size.cols }} ×
                {{ workbench.activeTerminal.size.rows }} · UTF-8
              </span>
              <AppButton
                v-if="
                  workbench.activeTerminal.kind === 'ssh' &&
                  workbench.activeGroup?.root.kind === 'terminal'
                "
                class="status-sftp-toggle"
                size="tiny"
                :variant="
                  workbench.activeTerminal.sftpOpen ? 'primary' : 'default'
                "
                :aria-pressed="!!workbench.activeTerminal.sftpOpen"
                :disabled="
                  !!workbench.drag ||
                  (!workbench.activeTerminal.sftpOpen &&
                    workbench.activeTerminal.state !== 'connected')
                "
                :title="t('files.sidebar.toggle')"
                @click="workbench.toggleSftp()"
              >
                SFTP
              </AppButton>
            </footer>
          </section>
        </main>
        <NewTabDialog
          v-model:open="newTabOpen"
          @connect="connect"
          @quick="quickConnect"
          @after-leave="restoreTerminalFocus"
        />
        <HostEditor
          v-model:open="editorOpen"
          :host="editing"
          :kind="editorKind"
          :initial-group-id="newHostGroup"
          :quick="quick"
          :duplicate="duplicate"
          @connect="ephemeral"
          @after-leave="restoreTerminalFocus"
        />
        <SaveWorkspaceDialog
          :group-id="savingGroupId"
          @close="savingGroupId = ''"
          @after-leave="restoreTerminalFocus"
        />
        <ChallengeDialog @after-leave="restoreTerminalFocus" />
        <SerialConnectDialog
          v-model:open="serialOpen"
          :save="serialSave"
          :host="editing"
          :duplicate="duplicate"
          :initial-group-id="newHostGroup"
          @connect="openSerial"
          @after-leave="restoreTerminalFocus"
        />
        <PasteDialog />
        <ConfirmationDialog />
      </div>
    </VaultGate>
  </NConfigProvider>
</template>

<style scoped>
.app-shell {
  height: 100dvh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 680px;
  background: var(--bg);
}
.workbench-content {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.terminal-area {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.terminal-stage {
  flex: 1;
  position: relative;
  min-height: 0;
  overflow: hidden;
  background: var(--terminal-background);
}
.status-bar {
  align-items: center;
  border-top: 1px solid var(--border);
  display: flex;
  gap: 20px;
  color: var(--muted);
  font-size: 11px;
  padding: 8px 14px;
}
.status-bar span:last-child {
  margin-left: auto;
}
.find-bar {
  display: flex;
  gap: 6px;
  justify-content: flex-end;
  padding: 8px;
  border-bottom: 1px solid var(--border);
}
.find-bar .ui-input {
  max-width: 300px;
}
.status-bar > .status-sftp-toggle {
  margin-left: auto;
}
.window-titlebar {
  grid-column: 1 / -1;
  grid-row: 1;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 12px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  color: var(--muted);
  font-size: 13px;
}
.window-drag-space {
  flex: 1;
  align-self: stretch;
}
.tabs-left {
  display: grid;
  grid-template-columns: 180px minmax(0, 1fr);
  grid-template-rows: 54px minmax(0, 1fr);
}
.tabs-left > .workbench-content {
  grid-column: 2;
  grid-row: 2;
}
.platform-mac .window-titlebar {
  padding-left: 88px;
}
</style>

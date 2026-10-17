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
import VaultLayout from "./layout/VaultLayout.vue";
import VaultGate from "./components/VaultGate.vue";
import WindowControls from "./components/WindowControls.vue";
import { isMac } from "./services/commands/registry";
import AppButton from "./components/ui/AppButton.vue";
import AppInput from "./components/ui/AppInput.vue";
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
import WorkspaceTabs from "./modules/workspace/WorkspaceTabs.vue";
import SftpWorkspace from "./modules/files/SftpWorkspace.vue";
import NewTabDialog from "./modules/hosts/NewTabDialog.vue";
import SaveWorkspaceDialog from "./modules/workspace/SaveWorkspaceDialog.vue";
import HostEditor from "./modules/hosts/HostEditor.vue";
import SerialConnectDialog from "./modules/terminal/SerialConnectDialog.vue";
import TerminalStage from "./modules/workspace/TerminalStage.vue";
import { useTerminalMenu } from "./modules/workspace/useTerminalMenu";
import { useTerminalClose } from "./modules/workspace/useTerminalClose";
import PasteDialog from "./modules/terminal/PasteDialog.vue";
import ChallengeDialog from "./modules/terminal/ChallengeDialog.vue";
import { useHosts } from "./modules/hosts/store";
import { usePortForwards } from "./modules/forwarding/store";
import { useSettings } from "./modules/settings/store";
import {
  useWorkspace,
  stateLabels,
  type TerminalOpenMode,
} from "./modules/workspace/store";
import { schemeForTab, terminalRegistry } from "./modules/terminal/registry";
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
const hosts = useHosts(),
  settings = useSettings(),
  workspace = useWorkspace();
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
  () => workspace.tabs.filter((t) => t.state === "connected").length,
);
const terminalBackground = computed(() =>
  workspace.active
    ? schemeForTab(workspace.active).colors.background
    : undefined,
);
const { naiveTheme, naiveThemeOverrides } = useAppAppearance();
let heartbeat: ReturnType<typeof setInterval> | undefined,
  unlisten: (() => void) | undefined,
  closingWindow = false;
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
const savingWorkspaceId = ref("");
function openNewTab() {
  newTabOpen.value = true;
}
function quickConnect(kind: HostKind = "ssh") {
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
  workspace.add("本地终端", { source: "local" });
}
function openSerial(config: SerialConfig) {
  workspace.add(config.path, { source: "serial", config });
}
function connect(host: HostProfile) {
  connectMany([host], "tabs");
}
function connectMany(profiles: HostProfile[], mode: TerminalOpenMode) {
  try {
    workspace.addSavedHosts(profiles, mode);
  } catch (e) {
    initError.value = errorMessage(e);
  }
}
function ephemeral(target: OpenTarget, title: string) {
  workspace.add(title, target);
}
const { closeTab, closePane } = useTerminalClose();
function perform(action: () => void | Promise<void>) {
  try {
    Promise.resolve(action()).catch((e) => {
      if (workspace.active) workspace.active.error = errorMessage(e);
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
    savingWorkspaceId.value = id;
  },
  focusTerminal,
  openFind: () => {
    findOpen.value = true;
  },
});
async function focusTerminal(id: string) {
  const group = workspace.groups.find((item) => item.id === id);
  if (!group) return;
  await nextTick();
  terminalRegistry.focus(group.focusedTerminalId);
}
function restoreTerminalFocus() {
  // 连接/认证弹窗的焦点陷阱退场后再聚焦，避免库恢复到已经隐藏的保险箱控件。
  if (!findOpen.value && workspace.active)
    terminalRegistry.focus(workspace.active.terminalId);
}
function closeFind() {
  findOpen.value = false;
  if (workspace.active) terminalRegistry.focus(workspace.active.terminalId);
}
function globalKey(event: KeyboardEvent) {
  if (!vaultReady.value) return;
  if ((event.target as HTMLElement).closest(".terminal-host")) return;
  commands.route(event, settings.value.keymap.bindings, false);
}
watch(
  () => settings.value,
  () => terminalRegistry.apply(),
  { deep: true },
);
watch(
  () => [workspace.activeId, workspace.active?.terminalId],
  () => {
    tabMenu.value.show = false;
    findOpen.value = false;
    query.value = "";
  },
  { flush: "post" },
);
watch(
  () => workspace.active?.schemeId,
  () => terminalRegistry.apply(),
);
onMounted(async () => {
  commands.register("workspace.newConnection", openNewTab);
  commands.register("workspace.newTab", openNewTab);
  commands.register("workspace.newLocalTerminal", openLocalTerminal);
  commands.register("workspace.closeTab", () => {
    if (workspace.activeGroup) void closeTab(workspace.activeGroup.id);
  });
  commands.register("workspace.nextTab", () => workspace.cycle(1));
  commands.register("workspace.previousTab", () => workspace.cycle(-1));
  commands.register("settings.open", () => perform(desktop.openSettingsWindow));
  commands.register("terminal.copy", () => {
    if (workspace.active)
      perform(() => terminalRegistry.copy(workspace.active!.terminalId));
  });
  commands.register("terminal.paste", () => {
    if (workspace.active)
      perform(() => terminalRegistry.paste(workspace.active!.terminalId));
  });
  commands.register("terminal.find", () => (findOpen.value = true));
  commands.register("terminal.scrollToBottom", () => {
    if (workspace.active) terminalRegistry.bottom(workspace.active.terminalId);
  });
  document.addEventListener("keydown", globalKey, true);
  try {
    unlisten = await desktop.onCloseRequested(async (event) => {
      if (closingWindow) return;
      event.preventDefault();
      if (
        (workspace.fileTransferring ||
          workspace.tabs.some((tab) => tab.sftpTransferring)) &&
        !(await confirmAction(
          t("关闭窗口将取消正在进行的 SFTP 传输，确定继续？"),
        ))
      )
        return;
      const running = workspace.tabs.filter(needsCloseConfirmation);
      if (
        (running.length > 0 || portForwards.hasActive) &&
        settings.value.confirmClose &&
        !(await confirmAction(
          [
            running.length
              ? t("{count} 个终端可能正在执行命令。", { count: running.length })
              : "",
            portForwards.hasActive ? t("端口转发正在运行。") : "",
            t("关闭窗口将断开全部连接，确定继续？"),
          ]
            .filter(Boolean)
            .join(" "),
        ))
      )
        return;
      try {
        await Promise.all(
          workspace.tabs.map((t) => terminalRegistry.dispose(t.terminalId)),
        );
        closingWindow = true;
        await desktop.closeWindow();
      } catch (e) {
        initError.value = errorMessage(e);
      }
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
    <VaultGate :initialize="loadUnlocked">
      <div
        class="app-shell"
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
        <WorkspaceTabs
          :tabs="workspace.workspaceTabs"
          :active-id="workspace.activeId"
          :position="settings.value.tabPosition"
          :connected="connected"
          @window-error="initError = $event"
          @activate="workspace.activateTab($event)"
          @focus-terminal="focusTerminal"
          @close="closeTab"
          @double-click="doubleClickTab"
          @menu="openTabMenu"
          @create="commands.execute('workspace.newTab')"
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
        <main class="workspace">
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
          <SftpWorkspace v-show="workspace.activeId === 'sftp'" />
          <!-- 终端宿主始终留在同一组件树内；页面与布局切换仅改变可见性。 -->
          <section
            v-show="!!workspace.active"
            class="terminal-workspace"
            :aria-label="t('终端工作区')"
          >
            <div v-if="findOpen && workspace.active" class="find-bar">
              <AppInput
                v-model="query"
                autofocus
                :aria-label="t('搜索终端历史')"
                :placeholder="t('搜索终端历史…')"
                @keydown.enter.prevent="
                  terminalRegistry.find(
                    workspace.active.terminalId,
                    query,
                    $event.shiftKey,
                  )
                "
                @keydown.esc="closeFind()"
              />
              <AppButton
                @click="
                  terminalRegistry.find(
                    workspace.active.terminalId,
                    query,
                    true,
                  )
                "
              >
                {{ t("上一个") }}
              </AppButton>
              <AppButton
                @click="
                  terminalRegistry.find(workspace.active.terminalId, query)
                "
              >
                {{ t("下一个") }}
              </AppButton>
              <AppButton
                class="icon-button"
                :aria-label="t('关闭搜索')"
                @click="closeFind()"
              >
                <NIcon :size="16" aria-hidden="true"><CloseOutline /></NIcon>
              </AppButton>
            </div>
            <NAlert v-if="workspace.active?.error" type="error">
              {{ workspace.active.error }}
            </NAlert>
            <div
              class="terminal-stage"
              :style="{ '--terminal-background': terminalBackground }"
            >
              <TerminalStage @close="closePane" @menu="openTerminalMenu" />
            </div>
            <footer v-if="workspace.active" class="status-bar">
              <span>
                {{
                  workspace.active.kind === "local"
                    ? t("本地")
                    : workspace.active.kind === "serial"
                      ? t("串口")
                      : workspace.active.kind === "telnet"
                        ? t("Telnet · 明文")
                        : "SSH"
                }}
                · {{ t(stateLabels[workspace.active.state]) }}
              </span>
              <span
                v-if="
                  workspace.active.kind === 'serial' &&
                  workspace.active.serialConfig
                "
              >
                {{ workspace.active.serialConfig.baudRate }} baud ·
                {{ workspace.active.serialConfig.charset }}
              </span>
              <span v-else>
                {{ workspace.active.size.cols }} ×
                {{ workspace.active.size.rows }} · UTF-8
              </span>
              <AppButton
                v-if="
                  workspace.active.kind === 'ssh' &&
                  workspace.activeGroup?.root.kind === 'terminal'
                "
                class="status-sftp-toggle"
                size="tiny"
                :variant="workspace.active.sftpOpen ? 'primary' : 'default'"
                :aria-pressed="!!workspace.active.sftpOpen"
                :disabled="
                  !!workspace.drag ||
                  (!workspace.active.sftpOpen &&
                    workspace.active.state !== 'connected')
                "
                :title="t('显示或隐藏此终端的 SFTP 侧栏')"
                @click="workspace.toggleSftp()"
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
          :group-id="savingWorkspaceId"
          @close="savingWorkspaceId = ''"
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
.workspace {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.terminal-workspace {
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
.tabs-left > .workspace {
  grid-column: 2;
  grid-row: 2;
}
.platform-mac .window-titlebar {
  padding-left: 88px;
}
</style>

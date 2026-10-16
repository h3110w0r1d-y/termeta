<script setup lang="ts">
import { closeContextMenuOnOutside } from "./components/ui/contextMenu";
import {
  NConfigProvider,
  NGlobalStyle,
  NAlert,
  NDropdown,
  zhCN,
  dateZhCN,
} from "naive-ui";
import { useAppAppearance } from "./layout/useAppAppearance";
import HomeLayout from "./layout/HomeLayout.vue";
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
import HostEditor from "./modules/hosts/HostEditor.vue";
import TerminalStage from "./modules/workspace/TerminalStage.vue";
import { useTerminalMenu } from "./modules/workspace/useTerminalMenu";
import { useTerminalClose } from "./modules/workspace/useTerminalClose";
import PasteDialog from "./modules/terminal/PasteDialog.vue";
import ChallengeDialog from "./modules/terminal/ChallengeDialog.vue";
import { useHosts } from "./modules/hosts/store";
import { useSettings } from "./modules/settings/store";
import { useWorkspace, stateLabels } from "./modules/workspace/store";
import { schemeForTab, terminalRegistry } from "./modules/terminal/registry";
import { needsCloseConfirmation } from "./modules/terminal/activity";
import { commands } from "./services/commands/registry";
import { desktop, errorMessage } from "./services/desktop/client";
import type { HostProfile, SshConfig } from "./services/desktop/types";
const hosts = useHosts(),
  settings = useSettings(),
  workspace = useWorkspace();
const newTabOpen = ref(false);
const editorOpen = ref(false),
  quick = ref(false),
  editing = ref<HostProfile>(),
  duplicate = ref(false),
  newHostGroup = ref<string | null>(null),
  initError = ref(""),
  findOpen = ref(false),
  query = ref("");
const vaultReady = ref(false);
async function loadUnlocked() {
  await Promise.all([hosts.load(), settings.load()]);
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
const { naiveTheme } = useAppAppearance();
let heartbeat: ReturnType<typeof setInterval> | undefined,
  unlisten: (() => void) | undefined,
  closingWindow = false;
function edit(host?: HostProfile, copy = false, groupId: string | null = null) {
  newHostGroup.value = groupId;
  editing.value = host;
  duplicate.value = copy;
  quick.value = false;
  editorOpen.value = true;
}
function openNewTab() {
  newTabOpen.value = true;
}
function quickConnect() {
  newHostGroup.value = null;
  editing.value = undefined;
  quick.value = true;
  duplicate.value = false;
  editorOpen.value = true;
}
function openLocalTerminal() {
  workspace.add("本地终端", { source: "local" });
}
function connect(host: HostProfile) {
  workspace.add(host.name, { source: "saved", profileId: host.id });
}
function ephemeral(config: SshConfig, title: string) {
  workspace.add(title, { source: "ephemeral", kind: "ssh", config });
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
  // 连接/认证弹窗的焦点陷阱退场后再聚焦，避免库恢复到已经隐藏的首页控件。
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
  commands.register("settings.open", () => workspace.openHome("settings"));
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
        !(await confirmAction("关闭窗口将取消正在进行的 SFTP 传输，确定继续？"))
      )
        return;
      const running = workspace.tabs.filter(needsCloseConfirmation);
      if (
        running.length > 0 &&
        settings.value.confirmClose &&
        !(await confirmAction(
          `${running.length} 个终端可能正在执行命令，关闭窗口将断开全部连接，确定继续？`,
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
  <NConfigProvider :theme="naiveTheme" :locale="zhCN" :date-locale="dateZhCN">
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
          <span data-tauri-drag-region>Termvexa</span>
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
            >{{ initError }}</NAlert
          >
          <HomeLayout
            @connect="connect"
            @local="openLocalTerminal"
            @edit="edit"
          />
          <SftpWorkspace v-show="workspace.activeId === 'sftp'" />
          <!-- 终端宿主始终留在同一组件树内；页面与布局切换仅改变可见性。 -->
          <section
            v-show="!!workspace.active"
            class="terminal-workspace"
            aria-label="终端工作区"
          >
            <div v-if="findOpen && workspace.active" class="find-bar">
              <AppInput
                v-model="query"
                autofocus
                aria-label="搜索终端历史"
                placeholder="搜索终端历史…"
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
                上一个</AppButton
              ><AppButton
                @click="
                  terminalRegistry.find(workspace.active.terminalId, query)
                "
              >
                下一个</AppButton
              ><AppButton
                class="icon-button"
                aria-label="关闭搜索"
                @click="closeFind()"
              >
                ✕
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
              <span
                >{{
                  workspace.active.target.source === "local" ? "本地" : "SSH"
                }}
                · {{ stateLabels[workspace.active.state] }}</span
              ><span
                >{{ workspace.active.size.cols }} ×
                {{ workspace.active.size.rows }} · UTF-8</span
              ><span>历史在断线后保留</span>
              <AppButton
                v-if="
                  workspace.active.target.source !== 'local' &&
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
                title="显示或隐藏此终端的 SFTP 侧栏"
                @click="workspace.toggleSftp()"
                >SFTP</AppButton
              >
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
          :initial-group-id="newHostGroup"
          :quick="quick"
          :duplicate="duplicate"
          @connect="ephemeral"
          @after-leave="restoreTerminalFocus"
        /><ChallengeDialog
          @after-leave="restoreTerminalFocus"
        /><PasteDialog /><ConfirmationDialog />
      </div>
    </VaultGate>
  </NConfigProvider>
</template>

<style>
/* 应用入口统一维护主题、基础规则、共享布局及跨组件的动态 DOM 适配。 */

:root {
  font-family:
    Inter,
    -apple-system,
    BlinkMacSystemFont,
    "Segoe UI",
    "Microsoft Yahei",
    sans-serif;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  --bg: #101014;
  --surface: #18181c;
  --surface-hover: #26262a;
  --border: #ffffff17;
  --text: #ffffffd1;
  --muted: #ffffff73;
  --accent: #63e2b7;
  --danger: #e88080;
  --input: #ffffff0f;
  color-scheme: dark;
}
* {
  box-sizing: border-box;
}
html,
body,
#app {
  width: 100%;
  height: 100%;
  overflow: hidden;
  overscroll-behavior: none;
}
body {
  margin: 0;
  -webkit-user-select: none;
  user-select: none;
}
/* 只开放实际可编辑或可复制的正文，xterm 继续使用自己的选区机制。 */
input,
textarea,
[contenteditable="true"],
.selectable-text,
[role="alert"],
.error,
.confirmation-message,
.fingerprint dd,
.paste-preview {
  -webkit-user-select: text;
  user-select: text;
}
h1,
h2,
h3,
p {
  margin-top: 0;
}
[data-appearance="light"] {
  --bg: #f7f7fa;
  --surface: #ffffff;
  --surface-hover: #f3f3f5;
  --border: #efeff5;
  --text: #333639;
  --muted: #767c82;
  --accent: #18a058;
  --danger: #d03050;
  --input: #fafafc;
  color-scheme: light;
}
@media (prefers-color-scheme: light) {
  [data-appearance="system"] {
    --bg: #f7f7fa;
    --surface: #ffffff;
    --surface-hover: #f3f3f5;
    --border: #efeff5;
    --text: #333639;
    --muted: #767c82;
    --accent: #18a058;
    --danger: #d03050;
    --input: #fafafc;
    color-scheme: light;
  }
}
.muted {
  color: var(--muted);
  font-size: 12px;
  line-height: 1.7;
}
.error {
  color: var(--danger);
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}
.button-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}
.form-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 18px 16px;
}
.span-2 {
  grid-column: span 2;
}
.status-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 100%;
  background: var(--muted);
  flex-shrink: 0;
  margin-right: 5px;
}
.status-dot.connected {
  background: var(--accent);
}
.status-dot.failed {
  background: var(--danger);
}
.status-dot.connecting,
.status-dot.awaitingAuth,
.status-dot.awaitingHostKey {
  background: #f2c97d;
}
.home-page {
  width: 100%;
  padding: 28px;
  max-width: 1540px;
  margin: 0 auto;
}

.page-heading {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 20px;
}
.page-heading h1 {
  margin: 9px 0;
  font-size: 26px;
  letter-spacing: -0.7px;
}

/* 适配固定的 xterm 6 DOM 渲染器：沿用原选区矩形，仅将透明选区放到文字上方。
   不拦截鼠标；升级渲染器时须复验这些 DOM 类名与图层关系。 */
.terminal-selection-overlay .xterm .xterm-screen > .xterm-selection {
  z-index: 3;
  pointer-events: none;
}
.terminal-selection-overlay .xterm .xterm-screen > .xterm-selection div {
  background-color: var(--terminal-selection-inactive);
}
.terminal-selection-overlay .xterm.focus .xterm-screen > .xterm-selection div {
  background-color: var(--terminal-selection-active);
}

/* 自定义名称可能很长，仅约束菜单文字尺寸，菜单皮肤/滚动/定位仍由Naive UI提供。 */
.terminal-context-menu .n-dropdown-option-body__label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>

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
@media (max-width: 900px) {
  .tabs-left {
    grid-template-columns: 145px minmax(0, 1fr);
  }
}
.platform-mac .window-titlebar {
  padding-left: 88px;
}
</style>

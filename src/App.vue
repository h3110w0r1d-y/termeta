<script setup lang="ts">
import {
  NConfigProvider,
  NGlobalStyle,
  NAlert,
  NMenu,
  NDropdown,
  darkTheme,
  zhCN,
  dateZhCN,
} from "naive-ui";
import AppButton from "./components/ui/AppButton.vue";
import AppInput from "./components/ui/AppInput.vue";
import AppScrollArea from "./components/ui/AppScrollArea.vue";
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
import CredentialsPage from "./modules/credentials/CredentialsPage.vue";
import WorkspaceTabs from "./modules/workspace/WorkspaceTabs.vue";
import SftpWorkspace from "./modules/workspace/SftpWorkspace.vue";
import HostsSidebar from "./modules/hosts/HostsSidebar.vue";
import HostEditor from "./modules/hosts/HostEditor.vue";
import SettingsDialog from "./modules/settings/SettingsDialog.vue";
import TerminalStage from "./modules/workspace/TerminalStage.vue";
import { terminalIds } from "./modules/workspace/layout";
import { builtinSchemes as builtinThemeOptions } from "./modules/settings/themes";
import PasteDialog from "./modules/terminal/PasteDialog.vue";
import ChallengeDialog from "./modules/terminal/ChallengeDialog.vue";
import { useHosts } from "./modules/hosts/store";
import { useSettings } from "./modules/settings/store";
import { useWorkspace, stateLabels, isActive } from "./modules/workspace/store";
import { schemeForTab, terminalRegistry } from "./modules/terminal/registry";
import { needsCloseConfirmation } from "./modules/terminal/activity";
import { commands } from "./services/commands/registry";
import { desktop, errorMessage } from "./services/desktop/client";
import type { HostProfile, SshConfig } from "./services/desktop/types";
const hosts = useHosts(),
  settings = useSettings(),
  workspace = useWorkspace();
const editorOpen = ref(false),
  quick = ref(false),
  editing = ref<HostProfile>(),
  duplicate = ref(false),
  initError = ref(""),
  findOpen = ref(false),
  query = ref("");
const connected = computed(
  () => workspace.tabs.filter((t) => t.state === "connected").length,
);
const terminalBackground = computed(() =>
  workspace.active
    ? schemeForTab(workspace.active).colors.background
    : undefined,
);
const systemAppearance = window.matchMedia("(prefers-color-scheme: dark)");
const systemDark = ref(systemAppearance.matches);
const naiveTheme = computed(() =>
  settings.value.appearance === "dark" ||
  (settings.value.appearance === "system" && systemDark.value)
    ? darkTheme
    : null,
);
function updateSystemAppearance(event: MediaQueryListEvent) {
  systemDark.value = event.matches;
}
let heartbeat: ReturnType<typeof setInterval> | undefined,
  unlisten: (() => void) | undefined,
  closingWindow = false;
function edit(host?: HostProfile, copy = false) {
  editing.value = host;
  duplicate.value = copy;
  quick.value = false;
  editorOpen.value = true;
}
function quickConnect() {
  editing.value = undefined;
  quick.value = true;
  duplicate.value = false;
  editorOpen.value = true;
}
function connect(host: HostProfile) {
  workspace.add(host.name, { source: "saved", profileId: host.id });
}
function ephemeral(config: SshConfig, title: string) {
  workspace.add(title, { source: "ephemeral", kind: "ssh", config });
}
async function closeTerminals(ids: string[], title: string) {
  if (ids.some((id) => workspace.closingTerminalIds.includes(id))) return;
  const terminals = workspace.tabs.filter((tab) =>
    ids.includes(tab.terminalId),
  );
  if (!terminals.length) return;
  workspace.closingTerminalIds.push(...ids);
  const restoreTabFocus = !!document.activeElement?.closest(
    "[data-workspace-tab], .workspace-tab-close",
  );
  try {
    if (
      terminals.some((tab) => tab.sftpTransferring) &&
      !(await confirmAction(
        "此终端有 SFTP 传输正在进行，关闭将取消传输，确定继续？",
      ))
    )
      return;
    const running = terminals.filter(needsCloseConfirmation);
    if (
      running.length > 0 &&
      settings.value.confirmClose &&
      !(await confirmAction(
        `“${title}”${terminals.length > 1 ? `中有 ${running.length} 个终端` : ""}可能正在执行命令，关闭将断开连接并释放终端历史，确定继续？`,
      ))
    )
      return;
    // 每个终端成功释放后才从布局删除，失败的窗格与历史保留以便重试。
    await Promise.all(
      terminals.map(async (tab) => {
        try {
          await terminalRegistry.dispose(tab.terminalId);
          workspace.dismiss(tab.sessionId ?? "");
          workspace.remove(tab.terminalId);
        } catch (error) {
          tab.error = errorMessage(error);
        }
      }),
    );
    await nextTick();
    if (restoreTabFocus)
      document
        .querySelector<HTMLElement>(
          '[data-workspace-tab][aria-selected="true"]',
        )
        ?.focus({ preventScroll: true });
    else if (workspace.active)
      terminalRegistry.focus(workspace.active.terminalId);
  } finally {
    workspace.closingTerminalIds = workspace.closingTerminalIds.filter(
      (id) => !ids.includes(id),
    );
  }
}
function closeTab(id: string) {
  const group = workspace.groups.find((item) => item.id === id);
  if (group)
    return closeTerminals(
      terminalIds(group.root),
      group.root.kind === "split"
        ? "Workspace"
        : (workspace.tabs.find(
            (tab) => tab.terminalId === group.focusedTerminalId,
          )?.title ?? "终端"),
    );
}
function closePane(id: string) {
  const terminal = workspace.tabs.find((tab) => tab.terminalId === id);
  if (terminal) return closeTerminals([id], terminal.title);
}
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
const tabMenu = ref({ show: false, id: "", x: 0, y: 0 });
// 同时约束主菜单和配色子菜单，滚动与边缘翻转交给Naive UI处理。
const terminalMenuProps = () => ({
  "data-ui-popup-open": "",
  class: "terminal-context-menu",
  style:
    "max-height: min(420px, calc(100dvh - 24px)); max-width: min(320px, calc(100dvw - 24px))",
});
const menuTab = computed(() =>
  workspace.tabs.find((tab) => tab.terminalId === tabMenu.value.id),
);
const tabMenuOptions = computed(() => [
  {
    label: `终端：${menuTab.value?.title ?? ""}`,
    key: "target",
    disabled: true,
  },
  { label: "搜索历史", key: "find" },
  {
    label: "终端配色",
    key: "theme",
    children: [
      { label: "跟随主机 / 默认配色", key: "scheme:default" },
      ...[...builtinThemeOptions, ...settings.value.customSchemes].map(
        (scheme) => ({ label: scheme.name, key: `scheme:${scheme.id}` }),
      ),
    ],
  },
  isActive(menuTab.value?.state ?? "closed")
    ? { label: "断开", key: "disconnect" }
    : { label: "重新连接", key: "reconnect" },
]);
function openTabMenu(event: MouseEvent | KeyboardEvent, id: string) {
  const group = workspace.groups.find((item) => item.id === id);
  if (group) openTerminalMenu(event, group.focusedTerminalId);
}
function openTerminalMenu(event: MouseEvent | KeyboardEvent, id: string) {
  event.preventDefault();
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  tabMenu.value = {
    show: true,
    id,
    x: event instanceof MouseEvent ? event.clientX : rect.left,
    y: event instanceof MouseEvent ? event.clientY : rect.bottom,
  };
}
async function selectTabAction(key: string) {
  tabMenu.value.show = false;
  const tab = menuTab.value;
  if (!tab) return;
  if (key === "disconnect") {
    if (
      tab.sftpTransferring &&
      !(await confirmAction("断开 SSH 将取消当前 SFTP 传输，确定继续？"))
    )
      return;
    perform(() => terminalRegistry.disconnect(tab.terminalId));
  } else if (key === "reconnect") perform(() => terminalRegistry.connect(tab));
  else if (key === "find") {
    workspace.focusTerminal(tab.terminalId);
    await nextTick();
    findOpen.value = true;
  } else if (key.startsWith("scheme:")) {
    tab.schemeId = key === "scheme:default" ? undefined : key.slice(7);
    terminalRegistry.apply();
  }
}
async function focusTerminal(id: string) {
  const group = workspace.groups.find((item) => item.id === id);
  if (!group) return;
  await nextTick();
  terminalRegistry.focus(group.focusedTerminalId);
}
function closeFind() {
  findOpen.value = false;
  if (workspace.active) terminalRegistry.focus(workspace.active.terminalId);
}
function globalKey(event: KeyboardEvent) {
  if ((event.target as HTMLElement).closest(".terminal-host")) return;
  commands.route(event, settings.value.keymap.bindings, false);
}
watch(
  () => settings.value,
  () => terminalRegistry.apply(),
  { deep: true },
);
watch(
  () => settings.value.appearance,
  (value) => {
    document.documentElement.dataset.appearance = value;
  },
  { immediate: true },
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
  systemAppearance.addEventListener("change", updateSystemAppearance);
  commands.register("workspace.newConnection", quickConnect);
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
  document.addEventListener("keydown", globalKey);
  try {
    await Promise.all([hosts.load(), settings.load()]);
    unlisten = await desktop.onCloseRequested(async (event) => {
      if (closingWindow) return;
      event.preventDefault();
      if (
        workspace.tabs.some((tab) => tab.sftpTransferring) &&
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
  systemAppearance.removeEventListener("change", updateSystemAppearance);
  document.removeEventListener("keydown", globalKey);
  if (heartbeat) clearInterval(heartbeat);
  unlisten?.();
});
</script>
<template>
  <NConfigProvider :theme="naiveTheme" :locale="zhCN" :date-locale="dateZhCN">
    <NGlobalStyle />
    <div
      class="app-shell"
      :class="{ 'tabs-left': settings.value.tabPosition === 'left' }"
    >
      <WorkspaceTabs
        :tabs="workspace.workspaceTabs"
        :active-id="workspace.activeId"
        :position="settings.value.tabPosition"
        :connected="connected"
        @activate="workspace.activateTab($event)"
        @focus-terminal="focusTerminal"
        @close="closeTab"
        @menu="openTabMenu"
        @create="commands.execute('workspace.newConnection')"
      />
      <NDropdown
        :show="tabMenu.show"
        :x="tabMenu.x"
        :y="tabMenu.y"
        :options="tabMenuOptions"
        :menu-props="terminalMenuProps"
        scrollable
        :flip="true"
        trigger="manual"
        display-directive="if"
        placement="bottom-start"
        @select="selectTabAction"
        @clickoutside="tabMenu.show = false"
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
            <div
              v-show="workspace.homeSection === 'hosts'"
              class="home-page-scroll"
            >
              <AppScrollArea label="主机内容">
                <HostsSidebar
                  @connect="connect"
                  @edit="edit"
                  @quick="quickConnect"
                />
              </AppScrollArea>
            </div>
            <AppScrollArea
              v-if="workspace.homeSection === 'credentials'"
              class="home-page-scroll"
              label="凭据内容"
            >
              <CredentialsPage />
            </AppScrollArea>
            <SettingsDialog
              v-if="workspace.homeSection === 'settings'"
              :embedded="true"
              :open="true"
            />
          </div>
        </section>
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
                terminalRegistry.find(workspace.active.terminalId, query, true)
              "
            >
              上一个</AppButton
            ><AppButton
              @click="terminalRegistry.find(workspace.active.terminalId, query)"
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
            <span>SSH · {{ stateLabels[workspace.active.state] }}</span
            ><span
              >{{ workspace.active.size.cols }} ×
              {{ workspace.active.size.rows }} · UTF-8</span
            ><span>历史在断线后保留</span>
            <AppButton
              class="status-sftp-toggle"
              size="tiny"
              :variant="workspace.active.sftpOpen ? 'primary' : 'default'"
              :aria-pressed="!!workspace.active.sftpOpen"
              :disabled="
                workspace.activeGroup?.root.kind !== 'terminal' ||
                !!workspace.drag ||
                (!workspace.active.sftpOpen &&
                  workspace.active.state !== 'connected')
              "
              :title="
                workspace.activeGroup?.root.kind === 'split'
                  ? 'Workspace 中不可用'
                  : '显示或隐藏此终端的 SFTP 侧栏'
              "
              @click="workspace.toggleSftp()"
              >SFTP</AppButton
            >
          </footer>
        </section>
      </main>
      <HostEditor
        v-model:open="editorOpen"
        :host="editing"
        :quick="quick"
        :duplicate="duplicate"
        @connect="ephemeral"
      /><ChallengeDialog /><PasteDialog /><ConfirmationDialog />
    </div>
  </NConfigProvider>
</template>

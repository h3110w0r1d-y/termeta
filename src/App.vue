<script setup lang="ts">
import { confirmAction } from "./components/ui/confirmation";
import { computed, onMounted, onBeforeUnmount, ref, watch } from "vue";
import ConfirmationDialog from "./components/ui/ConfirmationDialog.vue";
import HostsSidebar from "./modules/hosts/HostsSidebar.vue";
import HostEditor from "./modules/hosts/HostEditor.vue";
import SettingsDialog from "./modules/settings/SettingsDialog.vue";
import TerminalPane from "./modules/terminal/TerminalPane.vue";
import { builtinSchemes as builtinThemeOptions } from "./modules/settings/themes";
import PasteDialog from "./modules/terminal/PasteDialog.vue";
import ChallengeDialog from "./modules/terminal/ChallengeDialog.vue";
import { useHosts } from "./modules/hosts/store";
import { useSettings } from "./modules/settings/store";
import { useWorkspace, stateLabels, isActive } from "./modules/workspace/store";
import { terminalRegistry } from "./modules/terminal/registry";
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
  settingsOpen = ref(false),
  initError = ref(""),
  findOpen = ref(false),
  query = ref("");
const connected = computed(
  () => workspace.tabs.filter((t) => t.state === "connected").length,
);
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
  editorOpen.value = true;
}
function connect(host: HostProfile) {
  workspace.add(host.name, { source: "saved", profileId: host.id });
}
function ephemeral(config: SshConfig, title: string) {
  workspace.add(title, { source: "ephemeral", kind: "ssh", config });
}
async function closeTab(id: string) {
  const tab = workspace.tabs.find((t) => t.terminalId === id);
  if (!tab) return;
  if (
    isActive(tab.state) &&
    settings.value.confirmClose &&
    !(await confirmAction(`关闭“${tab.title}”并断开连接？终端历史将释放。`))
  )
    return;
  try {
    await terminalRegistry.dispose(id);
    workspace.dismiss(tab.sessionId ?? "");
    workspace.remove(id);
  } catch (e) {
    tab.error = errorMessage(e);
  }
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
function disconnectActive() {
  if (workspace.active)
    perform(() => terminalRegistry.disconnect(workspace.active!.terminalId));
}
function reconnectActive() {
  if (workspace.active)
    perform(() => terminalRegistry.connect(workspace.active!));
}
function tabKey(event: KeyboardEvent) {
  const index = workspace.tabs.findIndex(
    (t) => t.terminalId === workspace.activeId,
  );
  let next = index;
  if (event.key === "ArrowRight") next = (index + 1) % workspace.tabs.length;
  else if (event.key === "ArrowLeft")
    next = (index - 1 + workspace.tabs.length) % workspace.tabs.length;
  else if (event.key === "Home") next = 0;
  else if (event.key === "End") next = workspace.tabs.length - 1;
  else return;
  event.preventDefault();
  workspace.activeId = workspace.tabs[next]!.terminalId;
  requestAnimationFrame(() =>
    document
      .querySelector<HTMLButtonElement>("[role=tab][aria-selected=true]")
      ?.focus(),
  );
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
  () => workspace.activeId,
  () => {
    findOpen.value = false;
    query.value = "";
  },
);
watch(
  () => workspace.active?.schemeId,
  () => terminalRegistry.apply(),
);
onMounted(async () => {
  commands.register("workspace.newConnection", quickConnect);
  commands.register("workspace.closeTab", () => {
    if (workspace.active) void closeTab(workspace.active.terminalId);
  });
  commands.register("workspace.nextTab", () => workspace.cycle(1));
  commands.register("workspace.previousTab", () => workspace.cycle(-1));
  commands.register("settings.open", () => (settingsOpen.value = true));
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
      const active = workspace.tabs.filter((t) => isActive(t.state));
      if (
        active.length &&
        settings.value.confirmClose &&
        !(await confirmAction(
          `关闭窗口将断开 ${active.length} 个活动会话，确定继续？`,
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
  document.removeEventListener("keydown", globalKey);
  if (heartbeat) clearInterval(heartbeat);
  unlisten?.();
});
</script>
<template>
  <div class="app-shell">
    <HostsSidebar
      @connect="connect"
      @edit="edit"
      @quick="quickConnect"
      @settings="commands.execute('settings.open')"
    />
    <main class="workspace">
      <header class="workspace-header">
        <div>
          <span class="eyebrow">WORKSPACE</span>
          <h1>连接，让工作流畅发生。</h1>
        </div>
        <span class="connection-count"
          ><i class="status-dot" />{{ connected }} 个已连接</span
        ><button @click="commands.execute('workspace.newConnection')">
          ＋ 新建连接
        </button>
      </header>
      <div v-if="initError" class="app-error" role="alert">
        {{ initError
        }}<button class="icon-button" @click="initError = ''">✕</button>
      </div>
      <template v-if="workspace.tabs.length"
        ><nav
          class="tab-bar"
          role="tablist"
          aria-label="终端标签"
          @keydown="tabKey"
        >
          <div
            v-for="tab in workspace.tabs"
            :key="tab.terminalId"
            class="terminal-tab"
            :class="{ active: workspace.activeId === tab.terminalId }"
          >
            <button
              role="tab"
              :aria-selected="workspace.activeId === tab.terminalId"
              :tabindex="workspace.activeId === tab.terminalId ? 0 : -1"
              @click="workspace.activeId = tab.terminalId"
            >
              <i class="status-dot" :class="tab.state" />{{ tab.title }}</button
            ><button
              class="icon-button"
              :aria-label="'关闭 ' + tab.title"
              @click="closeTab(tab.terminalId)"
            >
              ×
            </button>
          </div>
          <button
            class="icon-button"
            aria-label="新建连接"
            @click="commands.execute('workspace.newConnection')"
          >
            ＋
          </button>
        </nav>
        <div v-if="workspace.active" class="terminal-toolbar">
          <span :class="{ error: workspace.active.state === 'failed' }">{{
            stateLabels[workspace.active.state]
          }}</span>
          <div class="toolbar-actions">
            <select
              v-model="workspace.active.schemeId"
              aria-label="当前终端配色"
            >
              <option :value="undefined">跟随默认配色</option>
              <option
                v-for="scheme in [
                  ...builtinThemeOptions,
                  ...settings.value.customSchemes,
                ]"
                :key="scheme.id"
                :value="scheme.id"
              >
                {{ scheme.name }}
              </option></select
            ><button
              title="搜索历史"
              @click="commands.execute('terminal.find')"
            >
              搜索</button
            ><button
              v-if="isActive(workspace.active.state)"
              @click="disconnectActive()"
            >
              断开</button
            ><button v-else @click="reconnectActive()">重新连接</button>
          </div>
        </div>
        <div v-if="findOpen && workspace.active" class="find-bar">
          <input
            v-model="query"
            autofocus
            placeholder="搜索终端历史…"
            @keydown.enter.prevent="
              terminalRegistry.find(
                workspace.active.terminalId,
                query,
                $event.shiftKey,
              )
            "
            @keydown.esc="closeFind()"
          /><button
            @click="
              terminalRegistry.find(workspace.active.terminalId, query, true)
            "
          >
            上一个</button
          ><button
            @click="terminalRegistry.find(workspace.active.terminalId, query)"
          >
            下一个</button
          ><button class="icon-button" @click="closeFind()">✕</button>
        </div>
        <div v-if="workspace.active?.error" class="session-error" role="alert">
          {{ workspace.active.error }}
        </div>
        <div class="terminal-stage">
          <TerminalPane
            v-for="tab in workspace.tabs"
            :key="tab.terminalId"
            :tab="tab"
            :active="workspace.activeId === tab.terminalId"
          />
        </div>
        <footer v-if="workspace.active" class="status-bar">
          <span>SSH · {{ stateLabels[workspace.active.state] }}</span
          ><span
            >{{ workspace.active.size.cols }} ×
            {{ workspace.active.size.rows }} · UTF-8</span
          ><span>历史在断线后保留</span>
        </footer></template
      >
      <section v-else class="welcome">
        <div class="welcome-glyph">›_</div>
        <span class="eyebrow">YOUR NEXT CONNECTION STARTS HERE</span>
        <h2>一个安静、专注的终端工作区</h2>
        <p>
          管理你的 SSH 主机，用熟悉的终端连接远方。<br />主机身份由你核验，凭据始终留在本机。
        </p>
        <div class="button-row">
          <button class="primary" @click="edit()">＋ 添加第一台主机</button
          ><button @click="commands.execute('workspace.newConnection')">
            快速连接
          </button>
        </div>
        <div class="welcome-features">
          <span>◈ 主机密钥核验</span><span>▤ 多标签会话</span
          ><span>◐ 独立终端配色</span>
        </div>
      </section>
    </main>
    <HostEditor
      v-model:open="editorOpen"
      :host="editing"
      :quick="quick"
      :duplicate="duplicate"
      @connect="ephemeral"
    /><SettingsDialog
      v-model:open="settingsOpen"
    /><ChallengeDialog /><PasteDialog /><ConfirmationDialog />
  </div>
</template>

import { clone } from "../../services/data/clone";
import { defineStore } from "pinia";
import { ref } from "vue";
import { desktop } from "../../services/desktop/client";
import { defaults, upgradeBindings } from "../../services/commands/registry";
import type {
  AppSettings,
  HostsViewSettings,
  HostSearchSettings,
} from "./model";
import { defaultSchemeId } from "./themes";
import { defaultTerminalFont, normalizeTerminalFont } from "./fonts";
export function defaultSettings(): AppSettings {
  return {
    schemaVersion: 1,
    appearance: "dark",
    tabPosition: "top",
    tabDoubleClickAction: "none",
    sftpShowHidden: false,
    hostsView: { includeChildren: false, expandedGroupIds: [] },
    hostSearch: {
      pinyin: true,
      fields: ["name", "address", "username", "tags", "group"],
    },
    fontFamily: defaultTerminalFont,
    fontSize: 14,
    scrollback: 5000,
    defaultSchemeId,
    hostSchemes: {},
    confirmClose: true,
    multilinePaste: true,
    keymap: { schemaVersion: 1, bindings: defaults() },
    customSchemes: [],
  };
}
export const useSettings = defineStore("settings", () => {
  const value = ref<AppSettings>(defaultSettings());
  async function load() {
    const saved = await desktop.getSettings();
    if (saved && Object.keys(saved).length)
      value.value = {
        ...defaultSettings(),
        ...saved,
        keymap: {
          schemaVersion: 1,
          bindings: upgradeBindings(saved.keymap?.bindings ?? defaults()),
        },
        fontFamily: normalizeTerminalFont(
          saved.fontFamily ?? defaultTerminalFont,
        ),
      };
  }
  const savingSftpHidden = ref(false);
  let pendingSave = Promise.resolve();
  // 整条设置JSON串行写入；在执行时合并即时偏好，避免设置页的旧草稿覆盖它。
  function persist(snapshot: () => AppSettings) {
    const request = pendingSave.then(async () => {
      const next = snapshot();
      await desktop.saveSettings(next);
      value.value = next;
    });
    pendingSave = request.catch(() => {});
    return request;
  }
  function save(next: AppSettings) {
    const snapshot = clone(next);
    return persist(() => ({
      ...snapshot,
      sftpShowHidden: value.value.sftpShowHidden,
      hostsView: clone(value.value.hostsView),
      hostSearch: clone(value.value.hostSearch),
    }));
  }
  async function setSftpShowHidden(show: boolean) {
    if (savingSftpHidden.value) return;
    savingSftpHidden.value = true;
    try {
      await persist(() => ({ ...clone(value.value), sftpShowHidden: show }));
    } finally {
      savingSftpHidden.value = false;
    }
  }
  function setHostsView(patch: Partial<HostsViewSettings>) {
    const snapshot = clone(patch);
    return persist(() => ({
      ...clone(value.value),
      hostsView: { ...clone(value.value.hostsView), ...snapshot },
    }));
  }
  function setHostSearch(patch: Partial<HostSearchSettings>) {
    const snapshot = clone(patch);
    return persist(() => ({
      ...clone(value.value),
      hostSearch: { ...clone(value.value.hostSearch), ...snapshot },
    }));
  }
  return {
    setHostSearch,
    value,
    load,
    save,
    savingSftpHidden,
    setSftpShowHidden,
    setHostsView,
  };
});

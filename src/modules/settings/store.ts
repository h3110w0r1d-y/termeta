import { clone } from "../../services/data/clone";
import { defineStore } from "pinia";
import { normalizeLanguage, setLanguage } from "../../i18n";
import { ref, watch } from "vue";
import { desktop } from "../../services/desktop/client";
import { defaults } from "../../services/commands/registry";
import type {
  AppSettings,
  SettingsPatch,
  HostsViewSettings,
  HostSearchSettings,
} from "./model";
import { defaultSchemeId } from "./themes";
import { defaultTerminalFont, normalizeTerminalFont } from "./fonts";
export function defaultSettings(): AppSettings {
  return {
    schemaVersion: 1,
    language: "system",
    appearance: "dark",
    primaryColor: "",
    tabPosition: "top",
    tabDoubleClickAction: "none",
    sftpShowHidden: false,
    detectSshSystem: true,
    hostsView: {
      includeChildren: false,
      expandedGroupIds: [],
      showHosts: false,
      displayMode: "card",
    },
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
    keymap: { schemaVersion: 2, bindings: defaults() },
    customSchemes: [],
  };
}
export const useSettings = defineStore("settings", () => {
  const value = ref<AppSettings>(defaultSettings());
  watch(() => value.value.language, setLanguage, {
    immediate: true,
    flush: "sync",
  });
  function normalize(saved: AppSettings): AppSettings {
    return {
      ...defaultSettings(),
      ...saved,
      language: normalizeLanguage(saved.language),
      // 旧版 hostsView 可能存在但缺少新增偏好，按子字段补齐默认值。
      hostsView: { ...defaultSettings().hostsView, ...saved.hostsView },
      fontFamily: normalizeTerminalFont(
        saved.fontFamily ?? defaultTerminalFont,
      ),
    };
  }
  function load() {
    const request = pendingSave.then(async () => {
      const saved = await desktop.getSettings();
      if (saved && Object.keys(saved).length) value.value = normalize(saved);
    });
    pendingSave = request.catch(() => {});
    return request;
  }
  const savingSftpHidden = ref(false);
  let pendingSave = Promise.resolve();
  // 单窗口读写共用队列，跨窗口合并由 Rust 的设置专用队列负责。
  function persist(patch: SettingsPatch) {
    const snapshot = clone(patch);
    const request = pendingSave.then(async () => {
      value.value = normalize(
        await desktop.saveSettings(snapshot, defaultSettings()),
      );
    });
    pendingSave = request.catch(() => {});
    return request;
  }
  function save(next: AppSettings) {
    const patch: Partial<AppSettings> = {};
    for (const key of Object.keys(next) as (keyof AppSettings)[]) {
      if (JSON.stringify(next[key]) !== JSON.stringify(value.value[key]))
        Object.assign(patch, { [key]: clone(next[key]) });
    }
    return persist(patch);
  }
  async function setSftpShowHidden(show: boolean) {
    if (savingSftpHidden.value) return;
    savingSftpHidden.value = true;
    try {
      await persist({ sftpShowHidden: show });
    } finally {
      savingSftpHidden.value = false;
    }
  }
  function setHostsView(patch: Partial<HostsViewSettings>) {
    return persist({ hostsView: clone(patch) });
  }
  function setHostSearch(patch: Partial<HostSearchSettings>) {
    return persist({ hostSearch: clone(patch) });
  }
  return {
    patch: persist,
    setHostSearch,
    value,
    load,
    save,
    savingSftpHidden,
    setSftpShowHidden,
    setHostsView,
  };
});

import { clone } from "../../services/data/clone";
import { defineStore } from "pinia";
import { ref } from "vue";
import { desktop } from "../../services/desktop/client";
import { defaults } from "../../services/commands/registry";
import type { AppSettings } from "./model";
import { defaultSchemeId } from "./themes";
import { defaultTerminalFont, normalizeTerminalFont } from "./fonts";
export function defaultSettings(): AppSettings {
  return {
    schemaVersion: 1,
    appearance: "dark",
    tabPosition: "top",
    tabDoubleClickAction: "none",
    sftpShowHidden: false,
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
  return { value, load, save, savingSftpHidden, setSftpShowHidden };
});

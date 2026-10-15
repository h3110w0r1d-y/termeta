import { clone } from "../../services/data/clone";
import { defineStore } from "pinia";
import { ref } from "vue";
import { desktop } from "../../services/desktop/client";
import { defaults } from "../../services/commands/registry";
import type { AppSettings } from "./model";
export function defaultSettings(): AppSettings {
  return {
    schemaVersion: 1,
    appearance: "dark",
    fontFamily: '"SFMono-Regular", Consolas, "Liberation Mono", monospace',
    fontSize: 14,
    scrollback: 5000,
    defaultSchemeId: "dark",
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
      value.value = { ...defaultSettings(), ...saved };
  }
  async function save(next: AppSettings) {
    await desktop.saveSettings(next);
    value.value = clone(next);
  }
  return { value, load, save };
});

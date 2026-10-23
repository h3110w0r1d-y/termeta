import { clone } from "@/services/data/clone";
import { defineStore } from "pinia";
import { setLanguage } from "@/i18n";
import { computed, ref, watch } from "vue";
import { desktop } from "@/services/desktop/client";
import type {
  AppSettings,
  SettingsPatch,
  HostsViewSettings,
  HostSearchSettings,
} from "./model";
import { defaultSettings, normalizeSettings } from "./state";

export const useSettings = defineStore("settings", () => {
  const value = ref<AppSettings>(defaultSettings());
  watch(() => value.value.language, setLanguage, {
    immediate: true,
    flush: "sync",
  });
  function load() {
    const request = pendingSave.then(async () => {
      const saved = await desktop.getSettings();
      if (saved && Object.keys(saved).length)
        value.value = normalizeSettings(saved);
    });
    pendingSave = request.catch(() => {});
    return request;
  }
  const savingSftpHidden = ref(false);
  const pendingSftpHidden = ref<boolean>();
  // 浏览过滤立即响应点击；持久化快照仍只保留已确认值，失败后自然回退。
  const sftpShowHidden = computed(
    () => pendingSftpHidden.value ?? value.value.sftpShowHidden,
  );
  let pendingSave = Promise.resolve();
  // 单窗口读写共用队列，跨窗口合并由 Rust 的设置专用队列负责。
  function persist(patch: SettingsPatch) {
    const snapshot = clone(patch);
    const request = pendingSave.then(async () => {
      value.value = normalizeSettings(
        await desktop.saveSettings(snapshot, defaultSettings()),
      );
    });
    pendingSave = request.catch(() => {});
    return request;
  }
  async function setSftpShowHidden(show: boolean) {
    if (savingSftpHidden.value) return;
    savingSftpHidden.value = true;
    pendingSftpHidden.value = show;
    try {
      await persist({ sftpShowHidden: show });
    } finally {
      pendingSftpHidden.value = undefined;
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
    savingSftpHidden,
    sftpShowHidden,
    setSftpShowHidden,
    setHostsView,
  };
});

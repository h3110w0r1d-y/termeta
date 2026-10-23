import { normalizeLanguage } from "@/i18n";
import { defaults } from "@/services/commands/registry";
import { defaultSchemeId } from "@/theme/schemes";
import type { AppSettings, HostsViewSettings } from "./model";
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
    sftpPosition: "right",
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
    terminalRightClickAction: "none",
    terminalMiddleClickAction: "none",
    keymap: { schemaVersion: 2, bindings: defaults() },
    customSchemes: [],
  };
}
type SavedSettings = Omit<Partial<AppSettings>, "appearance" | "hostsView"> & {
  appearance?: "system" | "dark" | "light";
  hostsView?: Partial<HostsViewSettings>;
};

// 接收 Rust 已校验、兼容归一后的快照；这里只补 UI 默认值，不替代持久化边界校验。
export function normalizeSettings(saved: SavedSettings): AppSettings {
  const initial = defaultSettings();
  return {
    ...initial,
    ...saved,
    // 兼容旧版浅色/跟随系统设置，不再影响应用外观。
    appearance: "dark",
    language: normalizeLanguage(saved.language),
    // 旧版 hostsView 可能存在但缺少新增偏好，按子字段补齐默认值。
    hostsView: { ...initial.hostsView, ...saved.hostsView },
    fontFamily: normalizeTerminalFont(saved.fontFamily ?? initial.fontFamily),
  };
}

// null 表示删除单个主机覆盖，跨窗口合并时保留其他主机的即时修改。
export function hostSchemePatch(
  next: Record<string, string>,
  previous: Record<string, string>,
) {
  return Object.fromEntries(
    [...new Set([...Object.keys(previous), ...Object.keys(next)])]
      .filter((id) => next[id] !== previous[id])
      .map((id) => [id, next[id] ?? null]),
  );
}

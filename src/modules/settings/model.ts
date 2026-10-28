import type { Language } from "@/i18n";
import type { Keymap } from "@/services/commands/types";
import type { SchemePreferences } from "@/theme/types";

export type { Binding, Modifier } from "@/services/commands/types";
export interface HostsViewSettings {
  includeChildren: boolean;
  showHosts: boolean;
  displayMode: "card" | "list";
  expandedGroupIds: string[];
}
export type HostSearchField =
  "name" | "address" | "username" | "tags" | "group";
export interface HostSearchSettings {
  pinyin: boolean;
  fields: HostSearchField[];
}
export type TerminalMouseAction = "none" | "paste";
export type HostSortMode = "recent" | "manual";
export interface AppSettings extends SchemePreferences {
  schemaVersion: 1;
  language: Language;
  primaryColor: string;
  // 保留旧持久化字段，应用外观统一使用深色。
  appearance: "dark";
  tabPosition: "top" | "left";
  tabDoubleClickAction: "none" | "close" | "duplicate" | "split";
  sftpShowHidden: boolean;
  sftpPosition: "left" | "right";
  detectSshSystem: boolean;
  hostSortMode: HostSortMode;
  hostsView: HostsViewSettings;
  hostSearch: HostSearchSettings;
  fontFamily: string;
  fontSize: number;
  scrollback: number;
  confirmClose: boolean;
  multilinePaste: boolean;
  terminalRightClickAction: TerminalMouseAction;
  terminalMiddleClickAction: TerminalMouseAction;
  keymap: Keymap;
}

// 即时偏好允许字段级更新，避免不同窗口覆盖同一对象中的其他字段。
export type SettingsPatch = Omit<
  Partial<AppSettings>,
  "hostsView" | "hostSearch" | "hostSchemes"
> & {
  hostsView?: Partial<HostsViewSettings>;
  hostSearch?: Partial<HostSearchSettings>;
  hostSchemes?: Record<string, string | null>;
};

import type { Binding } from "../../services/commands/types";

export interface Scheme {
  schemaVersion: 1;
  id: string;
  name: string;
  source: "builtin" | "user" | "imported";
  colors: {
    foreground: string;
    background: string;
    cursor: string;
    cursorAccent: string;
    selectionBackground: string;
    selectionForeground?: string;
    selectionInactiveBackground: string;
    ansi: string[];
    indexedOverrides?: Record<string, string>;
    scrollbar?: { normal?: string; hover?: string; active?: string };
  };
}
export type { Binding, Modifier } from "../../services/commands/types";
export interface HostsViewSettings {
  includeChildren: boolean;
  expandedGroupIds: string[];
}
export type HostSearchField =
  "name" | "address" | "username" | "tags" | "group";
export interface HostSearchSettings {
  pinyin: boolean;
  fields: HostSearchField[];
}
export interface AppSettings {
  schemaVersion: 1;
  primaryColor: string;
  appearance: "dark" | "light" | "system";
  tabPosition: "top" | "left";
  tabDoubleClickAction: "none" | "close" | "duplicate" | "split";
  sftpShowHidden: boolean;
  detectSshSystem: boolean;
  hostsView: HostsViewSettings;
  hostSearch: HostSearchSettings;
  fontFamily: string;
  fontSize: number;
  scrollback: number;
  defaultSchemeId: string;
  hostSchemes: Record<string, string>;
  confirmClose: boolean;
  multilinePaste: boolean;
  keymap: { schemaVersion: 1; bindings: Binding[] };
  customSchemes: Scheme[];
}

// 即时偏好允许字段级更新，避免不同窗口覆盖同一对象中的其他字段。
export type SettingsPatch = Omit<
  Partial<AppSettings>,
  "hostsView" | "hostSearch"
> & {
  hostsView?: Partial<HostsViewSettings>;
  hostSearch?: Partial<HostSearchSettings>;
};

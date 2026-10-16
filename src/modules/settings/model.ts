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
export type Modifier = "Primary" | "Control" | "Meta" | "Alt" | "Shift";
export interface Binding {
  id: string;
  commandId: string;
  scope: "application" | "terminal";
  context: "workspace" | "terminal";
  trigger:
    | { type: "key"; key: string; modifiers: Modifier[] }
    | {
        type: "pointer";
        button: "primary";
        event: "click";
        modifiers: Modifier[];
      };
  platform: "all";
  enabled: boolean;
}
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
  appearance: "dark" | "light" | "system";
  tabPosition: "top" | "left";
  tabDoubleClickAction: "none" | "close" | "duplicate" | "split";
  sftpShowHidden: boolean;
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

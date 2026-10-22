export interface Scheme {
  schemaVersion: 1;
  id: string;
  name: string;
  source: "builtin" | "user" | "imported";
  colors: {
    foreground: string;
    background: string;
    // 终端标签、状态栏与侧栏的辅助色；旧方案缺失时回退前景色。
    ui?: string;
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
export interface SchemePreferences {
  defaultSchemeId: string;
  hostSchemes: Record<string, string>;
  customSchemes: Scheme[];
}

// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "settings.language.system": "System",
  "settings.actions.open": "Open settings",
  "settings.detection.enabled":
    "Detect the operating system when connecting over SSH",
  "settings.detection.description":
    "Choose system icons automatically for saved SSH hosts. Manually selected icons take priority. Existing results are kept when this is turned off.",
  "settings.title": "Settings",
  "settings.windowTitle": "Termeta Settings",
  "settings.sections.general": "General",
  "settings.general.tabPosition": "Tab bar position",
  "settings.general.tabDoubleClick": "Double-click a tab",
  "settings.general.tabDoubleClickAction": "Tab double-click action",
  "settings.general.accentColor": "Accent color",
  "settings.terminal.font": "Terminal font",
  "settings.terminal.fontSize": "Font size",
  "settings.terminal.scrollback": "Scrollback lines",
  "settings.sections.systemDetection": "SSH system detection",
  "settings.sections.search": "Search",
  "settings.sections.security": "Security",
  "settings.sections.backup": "Backup / Restore",
  "settings.sections.shortcuts": "Shortcuts",
  "settings.tabPosition.top": "Top",
  "settings.tabPosition.left": "Left",
  "settings.security.confirmClose":
    "Confirm before closing a terminal that may be running a command",
  "settings.security.confirmMultilinePaste": "Confirm multiline paste",
  "settings.security.description":
    "Terminal output is not saved to disk. Hosts and credentials are encrypted on this device.",
  "settings.validation.accentColor": "Enter a 6-digit HEX color",
  "settings.validation.fontSize": "Font size must be between 10 and 32",
  "settings.doubleClick.none": "Do nothing",
  "settings.doubleClick.close": "Close tab",
  "settings.validation.terminalRange":
    "Font size must be 10–32 and scrollback must be 1,000–50,000 lines",
  "settings.validation.invalid": "Check your settings and try again",
  "settings.search.pinyin": "Pinyin search",
  "settings.search.description":
    "Changes are saved automatically. Names are always included in search.",
  "settings.search.address": "Address",
  "settings.general.language": "Language",
  "settings.languages.zhCN": "简体中文",
  "settings.languages.zhTW": "繁體中文",
  "settings.languages.jaJP": "日本語",
  "settings.languages.koKR": "한국어",
} as const;

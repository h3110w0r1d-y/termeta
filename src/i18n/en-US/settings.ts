// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "settings.terminal.convertChinesePunctuation":
    "Convert Chinese punctuation to English punctuation when typing",
  "settings.terminal.convertChinesePunctuationHint":
    "Convert Chinese punctuation typed or confirmed with an input method in the terminal. Pasted text is unchanged. Use the bottom status bar to override this for the current terminal until it is closed.",
  "settings.general.hostSortMode": "Host sort order",
  "settings.hostSort.recent": "Recently used first",
  "settings.hostSort.manual": "Manual order",
  "settings.hostSort.description":
    "When sorted by recent use, drag hosts to groups without reordering them. Switching back to manual order restores your previous order.",
  "settings.general.sftpPosition": "SFTP position",
  "settings.sftpPosition.left": "Left",
  "settings.sftpPosition.right": "Right",

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
  "settings.terminal.rightClickAction": "Terminal right-click action",
  "settings.terminal.middleClickAction": "Terminal middle-click action",
  "settings.mouseAction.none": "Do nothing",
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
  "settings.validation.accentColor": "Enter a 6-digit HEX color",
  "settings.validation.fontSize": "Font size must be between 10 and 32",
  "settings.doubleClick.none": "Do nothing",
  "settings.doubleClick.close": "Close tab",
  "settings.validation.terminalRange":
    "Font size must be 10–32 and scrollback must be 1,000–50,000 lines",
  "settings.validation.invalid": "Check your settings and try again",
  "settings.search.pinyin": "Pinyin search",
  "settings.search.address": "Address",
  "settings.general.language": "Language",
  "settings.languages.zhCN": "简体中文",
  "settings.languages.zhTW": "繁體中文",
  "settings.languages.jaJP": "日本語",
  "settings.languages.koKR": "한국어",
} as const;

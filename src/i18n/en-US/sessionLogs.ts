// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "sessionLogs.showHistory": "View scrollback",
  "sessionLogs.showScreen": "View final screen",
  "sessionLogs.invalidSnapshot": "The log display data is damaged or unsupported. Colors cannot be restored.",
  "sessionLogs.title": "Session logs",
  "sessionLogs.description":
    "Terminal content is saved when its tab closes or the app quits. Logs stay on this device and are excluded from backups.",
  "sessionLogs.empty":
    "No session logs yet. Close a terminal tab to save its content here.",
  "sessionLogs.noMatch": "No matching session logs.",
  "sessionLogs.search": "Search by name, username, host, or protocol",
  "sessionLogs.actions.clear": "Clear logs",
  "sessionLogs.actions.open": "View log",
  "sessionLogs.confirmClear":
    "Clear all local session logs? This cannot be undone. Saved hosts will be kept.",
  "sessionLogs.confirmDelete":
    "Delete the session log for “{name}”? This cannot be undone.",
  "sessionLogs.tabTitle": "Log · {name}",
  "sessionLogs.content": "Session log content, read only",
  "sessionLogs.emptyContent":
    "This terminal had no content to save when it closed.",
  "sessionLogs.timeRange": "Opened {started} · Saved {closed}",
} as const;

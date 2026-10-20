// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "shortcuts.triggers.click": "Click",
  "shortcuts.validation.conflict": "Shortcut conflict: {first} and {second}",
  "shortcuts.validation.invalidEntries": "Invalid shortcut entries.",
  "shortcuts.validation.invalidFields": "Invalid shortcut settings.",
  "shortcuts.validation.contextMismatch":
    "The shortcut is unavailable in this context.",
  "shortcuts.validation.linkRequiresMouse":
    "Use a mouse shortcut to open links.",
  "shortcuts.validation.unsupportedKey": "This key is not supported.",
  "shortcuts.validation.unsupportedTrigger":
    "This shortcut type is not supported.",
  "shortcuts.import.confirm":
    "Import {count} bindings and replace the current shortcuts?",
  "shortcuts.categories.label": "Shortcut categories",
  "shortcuts.search.placeholder": "Search actions or keys…",
  "shortcuts.actions.addBinding": "Add another shortcut",
  "shortcuts.actions.resetCommand": "Reset this action",
  "shortcuts.actions.deleteBinding": "Remove shortcut",
  "shortcuts.categories.application": "Application shortcuts",
  "shortcuts.categories.terminal": "Terminal shortcuts",
  "shortcuts.warning.remoteKeys":
    "Keys such as Ctrl+C will no longer be sent to the remote host if assigned to an application shortcut.",
  "shortcuts.hint.nativeSettings":
    "Cmd+, always opens Settings from the system menu, regardless of these bindings. You can record another shortcut for opening Settings.",
  "shortcuts.actions.resetAll": "Reset all",
  "shortcuts.actions.export": "Export settings",
  "shortcuts.actions.import": "Import settings",
  "shortcuts.validation.fileTooLarge": "The file is too large",
  "shortcuts.validation.unsupportedVersion":
    "Unsupported shortcut file version",
  "shortcuts.triggers.primaryClick": "Cmd / Ctrl + Click",
  "shortcuts.triggers.altClick": "Alt + Click",
  "shortcuts.triggers.shiftClick": "Shift + Click",
  "shortcuts.recording.hint": "Press keys; Esc to cancel",
  "shortcuts.recording.unbound": "Unassigned",
} as const;

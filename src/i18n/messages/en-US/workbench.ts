// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "workbench.actions.newTab": "New tab",
  "workbench.actions.newSshConnection": "New SSH connection",
  "workbench.actions.closeCurrentTab": "Close current tab",
  "workbench.actions.nextTab": "Next tab",
  "workbench.actions.previousTab": "Previous tab",
  "workbench.actions.newLocalTerminal": "New local terminal",
  "workbench.terminalCount": "{count} terminals",
  "workbench.close.confirmGroup":
    "{count} terminals in “{title}” may be running commands. Closing will disconnect them and clear their terminal history. Continue?",
  "workbench.close.confirmTerminal":
    "“{title}” may be running a command. Closing will disconnect it and clear its terminal history. Continue?",
  "workbench.layout.swapVertical": "Swap top and bottom",
  "workbench.layout.swapHorizontal": "Swap left and right",
  "workbench.actions.closeNamed": "Close {name}",
  "workbench.broadcast.disable":
    "Stop broadcasting ({broadcastCount} terminals)",
  "workbench.broadcast.commandExcluded":
    "Command terminals do not receive broadcast input",
  "workbench.actions.duplicateTab": "Duplicate tab",
  "workbench.tabs.label": "Application tabs",
  "workbench.broadcast.enable": "Broadcast input to {broadcastCount} terminals",
  "workbench.broadcast.unsupported":
    "Input broadcasting is not available for this terminal.",
  "workbench.actions.split": "Split session",
  "workbench.actions.newTabWithShortcut": "New tab (Cmd/Ctrl+T)",
  "workbench.broadcast.minimumTerminals":
    "Connect at least two interactive terminals first",
  "workbench.layout.resizeVertical": "Resize top and bottom terminals",
  "workbench.layout.resizeHorizontal": "Resize left and right terminals",
} as const;

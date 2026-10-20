// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "workspaces.title": "Workspace",
  "workspaces.errors.missingLayout": "This workspace has no saved layout.",
  "workspaces.actions.save": "Save workspace",
  "workspaces.actions.saveMenu": "Save workspace…",
  "workspaces.save.description":
    "Save the current layout and connections. Save again with the same name to update this workspace.",
  "workspaces.errors.workspaceOnly": "Only workspaces can be saved here.",
  "workspaces.errors.nameConflict":
    "A connection already uses this name. Only a workspace with the same name can be replaced.",
  "workspaces.save.confirmReplace":
    "This will replace the layout and connections in workspace “{name}”.",
  "workspaces.errors.deletedConnection":
    "A connection used by workspace “{name}” was deleted. Save the workspace again.",
  "workspaces.errors.singleConnection": "Open this workspace from the vault.",
  "workspaces.errors.invalidSplit": "This workspace layout is invalid.",
  "workspaces.errors.unsavedConnections":
    "Save all SSH, Telnet or serial connections before saving this workspace. Local terminals cannot be saved yet.",
  "workspaces.errors.layoutLimit":
    "This workspace layout exceeds the supported size.",
  "workspaces.errors.invalidFocus": "The selected workspace pane is invalid.",
  "workspaces.errors.invalidConnection":
    "A connection in this workspace is invalid.",
  "workspaces.actions.replace": "Replace workspace",
} as const;

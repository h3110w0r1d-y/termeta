// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "workspaces.title": "工作區",
  "workspaces.errors.missingLayout": "工作區版面配置缺失",
  "workspaces.actions.save": "儲存工作區",
  "workspaces.actions.saveMenu": "儲存工作區…",
  "workspaces.save.description":
    "儲存目前版面配置和連線。需要更改時，可再次儲存以替換同名工作區。",
  "workspaces.errors.workspaceOnly": "只能儲存工作區",
  "workspaces.errors.nameConflict": "名稱已被連線使用，只允許替換同名工作區",
  "workspaces.save.confirmReplace": "將替換工作區“{name}”的版面配置和連線。",
  "workspaces.errors.deletedConnection":
    "工作區“{name}”引用的連線已刪除，請重新儲存工作區",
  "workspaces.errors.singleConnection": "工作區不能作為單個終端連線",
  "workspaces.errors.invalidSplit": "工作區分割配置無效",
  "workspaces.errors.unsavedConnections":
    "工作區只能引用已儲存的 SSH、Telnet 或串列埠連線；請先儲存臨時連線配置，本機終端暫不支援儲存",
  "workspaces.errors.layoutLimit": "工作區版面配置超出限制",
  "workspaces.errors.invalidFocus": "工作區焦點無效",
  "workspaces.errors.invalidConnection": "工作區連線引用無效",
  "workspaces.actions.replace": "替換工作區",
} as const;

// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "workspaces.title": "工作区",
  "workspaces.errors.missingLayout": "工作区布局缺失",
  "workspaces.actions.save": "保存工作区",
  "workspaces.actions.saveMenu": "保存工作区…",
  "workspaces.save.description":
    "保存当前布局和连接。需要更改时，可再次保存以替换同名工作区。",
  "workspaces.errors.workspaceOnly": "只能保存工作区",
  "workspaces.errors.nameConflict": "名称已被连接使用，只允许替换同名工作区",
  "workspaces.save.confirmReplace": "将替换工作区“{name}”的布局和连接。",
  "workspaces.errors.deletedConnection":
    "工作区“{name}”引用的连接已删除，请重新保存工作区",
  "workspaces.errors.singleConnection": "工作区不能作为单个终端连接",
  "workspaces.errors.invalidSplit": "工作区分割配置无效",
  "workspaces.errors.unsavedConnections":
    "工作区只能引用已保存的 SSH、Telnet 或串口连接；请先保存临时连接配置，本地终端暂不支持保存",
  "workspaces.errors.layoutLimit": "工作区布局超出限制",
  "workspaces.errors.invalidFocus": "工作区焦点无效",
  "workspaces.errors.invalidConnection": "工作区连接引用无效",
  "workspaces.actions.replace": "替换工作区",
} as const;

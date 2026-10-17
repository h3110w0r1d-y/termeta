// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "workbench.actions.newTab": "新建标签页",
  "workbench.actions.newSshConnection": "新建 SSH 连接",
  "workbench.actions.closeCurrentTab": "关闭当前标签",
  "workbench.actions.nextTab": "下一个标签",
  "workbench.actions.previousTab": "上一个标签",
  "workbench.actions.newLocalTerminal": "新建本地终端",
  "workbench.terminalCount": "{count} 个终端",
  "workbench.close.confirmGroup":
    "“{title}”中有 {count} 个终端可能正在执行命令。关闭将断开连接并保存会话日志，是否继续？",
  "workbench.close.confirmTerminal":
    "“{title}”可能正在执行命令。关闭将断开连接并保存会话日志，是否继续？",
  "workbench.layout.swapVertical": "交换上下区域",
  "workbench.layout.swapHorizontal": "交换左右区域",
  "workbench.actions.closeNamed": "关闭 {name}",
  "workbench.broadcast.disable": "关闭广播（{broadcastCount} 个终端）",
  "workbench.broadcast.commandExcluded": "命令终端不接收广播",
  "workbench.actions.duplicateTab": "复制标签页",
  "workbench.tabs.label": "应用标签页",
  "workbench.broadcast.enable": "广播输入到 {broadcastCount} 个终端",
  "workbench.broadcast.unsupported": "当前终端暂不支持输入广播。",
  "workbench.actions.split": "拆分会话",
  "workbench.actions.newTabWithShortcut": "新建标签页 (Cmd/Ctrl+T)",
  "workbench.broadcast.minimumTerminals": "请先连接至少两个交互终端",
  "workbench.layout.resizeVertical": "调整上下终端高度",
  "workbench.layout.resizeHorizontal": "调整左右终端宽度",
} as const;

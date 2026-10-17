// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "window.controls.label": "窗口操作",
  "window.controls.minimizeLabel": "最小化窗口",
  "window.controls.minimize": "最小化",
  "window.controls.maximizeLabel": "最大化或还原窗口",
  "window.controls.maximize": "最大化 / 还原",
  "window.controls.close": "关闭窗口",
  "window.close.confirmTransfers":
    "关闭窗口将取消正在进行的 SFTP 传输，确定继续？",
  "window.close.confirmSessions": "关闭窗口将断开全部连接，确定继续？",
  "window.close.runningTerminals": "{count} 个终端可能正在执行命令。",
  "window.close.activeForwarding": "端口转发正在运行。",
} as const;

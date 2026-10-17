// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "sessionLogs.showHistory": "查看历史内容",
  "sessionLogs.showScreen": "查看关闭时屏幕",
  "sessionLogs.invalidSnapshot": "日志显示数据损坏或格式不受支持，无法还原颜色。",
  "sessionLogs.title": "会话日志",
  "sessionLogs.description":
    "关闭终端标签或退出软件时保存终端内容，仅保存在本机，不参与备份。",
  "sessionLogs.empty": "尚无会话日志。关闭终端标签后可在此回看。",
  "sessionLogs.noMatch": "没有匹配的会话日志。",
  "sessionLogs.search": "搜索名称、账号、主机或协议",
  "sessionLogs.actions.clear": "清空日志",
  "sessionLogs.actions.open": "查看日志",
  "sessionLogs.confirmClear":
    "清空全部本机会话日志？此操作无法撤销，不会删除主机配置。",
  "sessionLogs.confirmDelete": "删除“{name}”的会话日志？此操作无法撤销。",
  "sessionLogs.tabTitle": "日志 · {name}",
  "sessionLogs.content": "会话日志正文，只读",
  "sessionLogs.emptyContent": "该终端关闭时没有可保存的内容。",
  "sessionLogs.timeRange": "打开于 {started} · 保存于 {closed}",
} as const;

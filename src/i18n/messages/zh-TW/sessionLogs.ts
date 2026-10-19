// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "sessionLogs.showHistory": "檢視歷史內容",
  "sessionLogs.showScreen": "檢視關閉時螢幕",
  "sessionLogs.invalidSnapshot":
    "日誌顯示資料損壞或格式不受支援，無法還原顏色。",
  "sessionLogs.title": "會話日誌",
  "sessionLogs.description":
    "關閉終端標籤或退出軟體時儲存終端內容，僅儲存在本機，不參與備份。",
  "sessionLogs.empty": "尚無會話日誌。關閉終端標籤後可在此回看。",
  "sessionLogs.noMatch": "沒有匹配的會話日誌。",
  "sessionLogs.search": "搜尋名稱、賬號、主機或協議",
  "sessionLogs.actions.clear": "清空日誌",
  "sessionLogs.actions.open": "檢視日誌",
  "sessionLogs.confirmClear":
    "清空全部本機會話日誌？此操作無法撤銷，不會刪除主機配置。",
  "sessionLogs.confirmDelete": "刪除“{name}”的會話日誌？此操作無法撤銷。",
  "sessionLogs.tabTitle": "日誌 · {name}",
  "sessionLogs.content": "會話日誌正文，只讀",
  "sessionLogs.emptyContent": "該終端關閉時沒有可儲存的內容。",
  "sessionLogs.timeRange": "開啟於 {started} · 儲存於 {closed}",
} as const;

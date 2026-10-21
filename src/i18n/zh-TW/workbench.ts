// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "workbench.sidebar.themes": "主題",
  "workbench.sidebar.toggle": "切換工具側欄",
  "workbench.sidebar.label": "終端工具",
  "workbench.sidebar.themeHint":
    "字型與字級套用至所有終端。選擇配色會讓目前主機繼承預設，並更新全域配色。",
  "workbench.sidebar.noTerminal": "請啟用一個已連線的終端以使用片段。",
  "workbench.sidebar.execute": "執行",

  "workbench.actions.newTab": "新建標籤頁",
  "workbench.actions.newSshConnection": "新建 SSH 連線",
  "workbench.actions.closeCurrentTab": "關閉目前標籤",
  "workbench.actions.nextTab": "下一個標籤",
  "workbench.actions.previousTab": "上一個標籤",
  "workbench.actions.newLocalTerminal": "新建本機終端",
  "workbench.terminalCount": "{count} 個終端",
  "workbench.close.confirmGroup":
    "“{title}”中有 {count} 個終端可能正在執行命令。關閉將斷開連線並儲存會話日誌，是否繼續？",
  "workbench.close.confirmTerminal":
    "“{title}”可能正在執行命令。關閉將斷開連線並儲存會話日誌，是否繼續？",
  "workbench.layout.swapVertical": "交換上下區域",
  "workbench.layout.swapHorizontal": "交換左右區域",
  "workbench.actions.closeNamed": "關閉 {name}",
  "workbench.broadcast.disable": "關閉廣播（{broadcastCount} 個終端）",
  "workbench.broadcast.commandExcluded": "命令終端不接收廣播",
  "workbench.actions.duplicateTab": "複製標籤頁",
  "workbench.tabs.label": "應用標籤頁",
  "workbench.broadcast.enable": "廣播輸入到 {broadcastCount} 個終端",
  "workbench.broadcast.unsupported": "目前終端暫不支援輸入廣播。",
  "workbench.actions.split": "拆分會話",
  "workbench.actions.newTabWithShortcut": "新建標籤頁 (Cmd/Ctrl+T)",
  "workbench.broadcast.minimumTerminals": "請先連線至少兩個互動終端",
  "workbench.layout.resizeVertical": "調整上下終端高度",
  "workbench.layout.resizeHorizontal": "調整左右終端寬度",
} as const;

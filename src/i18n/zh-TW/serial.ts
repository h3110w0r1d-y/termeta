// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "serial.errors.missingConfiguration": "串列埠配置缺失",
  "serial.connect.title": "串列埠連線",
  "serial.form.namePlaceholder": "例如：開發板串列埠",
  "serial.form.stopBits": "停止位元",
  "serial.parity.even": "偶同位檢查",
  "serial.actions.refresh": "重新整理串列埠",
  "serial.parity.odd": "奇同位檢查",
  "serial.form.encoding": "字元集",
  "serial.form.tagsPlaceholder": "開發板, 除錯",
  "serial.actions.open": "開啟串列埠",
  "serial.form.dataBits": "資料位元",
  "serial.actions.newHost": "新建串列埠主機",
  "serial.empty":
    "未發現串列埠。請連線裝置後重新整理，或手動輸入裝置名稱。Linux 使用者還需檢查串列埠存取權限。",
  "serial.form.parity": "驗證",
  "serial.form.baudRate": "鮑率",
  "serial.validation.baudRate": "鮑率須為 1–4000000 的整數",
  "serial.form.flowControl": "流量控制",
  "serial.actions.editHost": "編輯串列埠主機",
  "serial.validation.device":
    "請輸入串列埠裝置路徑或名稱，最長1024位元組且不能含控制字元",
  "serial.form.devicePlaceholder": "選擇或輸入串列埠裝置路徑/名稱",
  "serial.form.baudRatePlaceholder": "選擇或輸入鮑率（預設9600）",
  "serial.actions.reopen": "重新開啟串列埠，保留終端歷史",
  "serial.form.advanced": "進階設定",
} as const;

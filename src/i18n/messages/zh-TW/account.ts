export default {
  "account.oauth.confirmationCode": "瀏覽器一次性確認碼",
  "account.oauth.codeDescription":
    "若瀏覽器無法開啟 Termeta，請從授權成功頁面複製一次性確認碼，填入下方。",
  "account.oauth.manualFallback": "未自動返回？手動輸入確認碼",
  "account.oauth.returnHint":
    "若瀏覽器詢問是否開啟 Termeta，請允許；若未自動返回，可按頁面上的「返回 Termeta」。請保持此視窗開啟，等待完成。",
  "account.oauth.codeSecurity":
    "僅填寫本人在此裝置發起的授權確認碼；請勿分享確認碼或代他人填寫。",
  "account.oauth.submitCode": "提交確認碼",
  "account.oauth.codeSubmitted":
    "確認碼已提交，正在等待驗證。若尚未完成，請保留此視窗。",
  "account.oauth.validationCode":
    "請輸入瀏覽器顯示的 32 位一次性確認碼，僅包含字母、數字、底線或短橫線。",

  "account.actions.clearLogin": "登出 / 清除本機登入",
  "account.title": "帳號",
  "account.description":
    "登入官方 Termeta 帳號，可使用電子郵件密碼、GitHub 或 Google。本機終端無需帳號即可使用。",
  "account.encryptionHint":
    "帳號登入不會解鎖保險庫，也不會取代保險庫密碼或備份加密密碼。",
  "account.admin": "管理員",
  "account.createdAt": "註冊時間：{date}",
  "account.expiresAt": "工作階段到期：{date}",
  "account.signedOut": "目前未登入。",
  "account.actions.refresh": "重新整理帳號狀態",
  "account.actions.logout": "登出",
  "account.actions.login": "登入",
  "account.actions.register": "註冊",
  "account.actions.retry": "重新讀取帳號",
  "account.actions.unlink": "解除綁定",
  "account.actions.linkProvider": "綁定 {provider}",
  "account.actions.loginProvider": "使用 {provider} 登入",
  "account.actions.retryProviders": "重新載入登入方式",
  "account.actions.cancelLogin": "取消等待",
  "account.actions.openSettings": "帳號與設定",
  "account.identities.title": "登入方式",
  "account.identities.description":
    "綁定前請在瀏覽器確認第三方帳號。相同電子郵件不會自動合併帳號。",
  "account.identities.password": "電子郵件與密碼",
  "account.identities.lastMethod": "至少需要保留一種登入方式。",
  "account.unlink.confirm":
    "確定解除綁定 {provider}？解除後無法再透過該帳號登入 Termeta。",
  "account.logout.confirm":
    "確定登出官方帳號？本機保險庫和終端工作階段不受影響。",
  "account.oauth.completed": "授權已完成，帳號狀態已更新。",
  "account.oauth.cancelled": "已取消本次瀏覽器授權。",
  "account.oauth.waiting":
    "請在系統瀏覽器中完成 {provider} 授權，隨後將自動返回 Termeta 完成驗證。",
  "account.oauth.deadline": "等待截止：{date}",
  "account.errors.expired": "授權已過期，請重新開始。",
  "account.errors.denied": "授權未完成，請重新開始或使用其他方式登入。",
  "account.form.email": "電子郵件",
  "account.form.password": "密碼",
  "account.form.confirmation": "確認密碼",
  "account.validation.email": "請輸入有效的電子郵件地址。",
  "account.validation.password": "密碼須為 12–256 個字元。",
  "account.validation.confirmation": "兩次輸入的密碼不一致。",
} as const;

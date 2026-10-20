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
  "account.title": "帳號",
  "account.description":
    "選擇官方帳號模式後，保險庫會自動完整同步至 termeta.io；本機模式無需帳號。",
  "account.actions.logout": "登出",
  "account.actions.loginProvider": "使用 {provider} 登入",
  "account.actions.retryProviders": "重新載入登入方式",
  "account.actions.cancelLogin": "取消等待",
  "account.actions.openSettings": "帳號與設定",
  "account.logout.confirm":
    "確定登出官方帳號並切換至本機模式？本機資料和終端工作階段將保留，官方自動同步將停止。",
  "account.oauth.waiting":
    "請在系統瀏覽器中完成 {provider} 授權，隨後將自動返回 Termeta 完成驗證。",
  "account.oauth.deadline": "等待截止：{date}",
  "account.errors.expired": "授權已過期，請重新開始。",
  "account.errors.denied": "授權未完成，請重新開始或使用其他方式登入。",
  "account.mode.title": "軟體啟動模式",
  "account.mode.description":
    "登入官方帳號，將保險庫自動同步至 termeta.io；或使用本機離線模式，自行管理資料。",
  "account.mode.official": "官方帳號模式",
  "account.mode.local": "本機 / 離線模式",
  "account.mode.logoutLocal": "登出帳號並使用本機模式",
  "account.browser.description":
    "帳號註冊、登入及登入方式管理皆在瀏覽器中的 termeta.io 完成。",
  "account.sync.unlockTitle": "解鎖雲端保險庫",
  "account.sync.unlockDescription":
    "雲端已有資料。請輸入其保險庫密碼，驗證成功後合併本機資料並繼續。密碼錯誤時無法進入；可登出帳號保留本機資料，改用本機模式。",
  "account.sync.setupDescription":
    "請設定保險庫密碼。此密碼同時保護本機與官方雲端資料，請牢記；帳號登入無法復原遺忘的保險庫密碼。",
  "account.sync.existingDescription":
    "請輸入目前本機保險庫密碼，啟用官方自動同步。之後官方雲端與本機保險庫使用相同密碼。",
  "account.sync.description":
    "官方雲端自動完整同步已啟用，使用與本機保險庫相同的主金鑰。登出帳號會保留本機資料，並停止官方同步。",
  "account.sync.mainWindowRequired":
    "請返回 Termeta 主視窗輸入保險庫密碼，完成雲端解密與合併。驗證完成前無法進入工作台。",
  "account.sync.lastSynced": "上次同步：{date}",
  "account.sync.now": "立即同步",
  "account.sync.continue": "驗證並繼續",
  "account.sync.localUnlockTitle": "先解鎖本機保險庫",
  "account.sync.localUnlockDescription":
    "請輸入目前本機保險庫密碼，以保留並合併本機資料。若雲端密碼已在其他裝置變更，本機舊密碼可能與雲端不同；本機解鎖後再驗證雲端密碼，無需登出帳號。",
} as const;

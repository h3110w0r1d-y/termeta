// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "vault.validation.passwordRequired": "請輸入保險庫密碼",
  "vault.validation.passwordMismatch": "兩次輸入的密碼須一致",
  "vault.setup.title": "設定資料保險庫密碼",
  "vault.unlock.title": "解鎖資料保險庫",
  "vault.setup.method": "建立保險庫方式",
  "vault.setup.empty": "建立空保險庫",
  "vault.setup.cloud": "從雲端備份初始化",
  "vault.setup.passwordPlaceholder": "設定密碼（至少12個字元）",
  "vault.form.password": "保險庫密碼",
  "vault.form.confirmPassword": "確認密碼",
  "vault.actions.createAndUnlock": "建立並解鎖",
  "vault.actions.unlock": "解鎖",
  "vault.validation.passwordLength": "密碼至少12個字元，且長度不超過1024位元組",
  "vault.setup.warning":
    "主機和認證資訊會加密儲存在本機。請牢記密碼：如果自動解鎖失效且忘記密碼，資料將無法恢復。",
  "vault.unlock.automaticProgress":
    "正在嘗試自動解鎖。如未成功，請輸入保險庫密碼。",
  "vault.password.title": "修改保險庫密碼",
  "vault.password.warning":
    "請牢記新密碼。已有本機副本仍需舊密碼，雲端備份密碼保持不變。",
  "vault.password.old": "舊密碼",
  "vault.password.new": "新密碼",
  "vault.password.confirm": "確認新密碼",
  "vault.password.description":
    "修改密碼將重新加密本機儲存的主機、認證資訊、設定和歷史。",
  "vault.password.success": "保險庫密碼已修改。",
  "vault.password.progress": "正在重新加密，請稍候…",
  "vault.password.submit": "確認修改",
  "vault.password.validation.old": "請輸入舊密碼，長度不能超過1024位元組",
  "vault.password.validation.length":
    "新密碼至少12個字元，長度不能超過1024位元組",
  "vault.password.validation.unchanged": "新密碼不能與舊密碼相同",
  "vault.password.validation.mismatch": "兩次輸入的新密碼須一致",
} as const;

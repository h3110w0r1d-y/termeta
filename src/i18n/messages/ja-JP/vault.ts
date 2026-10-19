// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "vault.validation.passwordRequired": "保管庫のパスワードを入力してください",
  "vault.validation.passwordMismatch": "パスワードが一致しません。",
  "vault.setup.title": "保管庫のパスワードを作成",
  "vault.unlock.title": "保管庫をロック解除",
  "vault.setup.method": "保管庫を作成",
  "vault.setup.empty": "空の保管庫を作成",
  "vault.setup.cloud": "クラウドバックアップから復元",
  "vault.setup.passwordPlaceholder": "パスワード（12文字以上）",
  "vault.form.password": "保管庫のパスワード",
  "vault.form.confirmPassword": "パスワードの確認",
  "vault.actions.createAndUnlock": "作成してロック解除",
  "vault.actions.unlock": "ロック解除",
  "vault.validation.passwordLength":
    "12文字以上、1024バイト以内で入力してください。",
  "vault.setup.warning":
    "ホストと認証情報はこのデバイスで暗号化されます。パスワードを安全に保管してください。自動ロック解除が利用できず、パスワードを忘れた場合、データは復元できません。",
  "vault.unlock.automaticProgress":
    "自動ロック解除を試行しています。利用できない場合は保管庫のパスワードを入力してください。",
  "vault.password.title": "保管庫のパスワードを変更",
  "vault.password.warning":
    "新しいパスワードを忘れないでください。既存のローカルコピーには引き続き古いパスワードが必要です。クラウドバックアップのパスワードは変わりません。",
  "vault.password.old": "現在のパスワード",
  "vault.password.new": "新しいパスワード",
  "vault.password.confirm": "新しいパスワードの確認",
  "vault.password.description":
    "パスワードを変更すると、このデバイスに保存されたホスト、認証情報、設定、履歴を再暗号化します。",
  "vault.password.success": "保管庫のパスワードを変更しました。",
  "vault.password.progress": "データを再暗号化しています。お待ちください…",
  "vault.password.submit": "パスワードを変更",
  "vault.password.validation.old":
    "現在のパスワードを入力してください（1,024バイト以内）",
  "vault.password.validation.length":
    "12文字以上、1,024バイト以内で入力してください",
  "vault.password.validation.unchanged":
    "現在とは異なるパスワードを指定してください",
  "vault.password.validation.mismatch": "新しいパスワードが一致しません",
} as const;

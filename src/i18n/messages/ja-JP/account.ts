export default {
  "account.oauth.confirmationCode": "ブラウザーのワンタイム確認コード",
  "account.oauth.codeDescription":
    "ブラウザーから Termeta を開けない場合は、認証完了ページのワンタイム確認コードをコピーして下に入力してください。",
  "account.oauth.manualFallback": "自動で戻らない場合は確認コードを入力",
  "account.oauth.returnHint":
    "ブラウザーに確認が表示されたら Termeta を開くことを許可してください。自動で戻らない場合は、ページの「Termeta に戻る」を選んでください。検証が完了するまで、このウィンドウを開いたままにしてください。",
  "account.oauth.codeSecurity":
    "この端末で自分が開始した認証のコードだけを入力してください。コードを他人に伝えたり、他人の代わりに入力したりしないでください。",
  "account.oauth.submitCode": "確認コードを送信",
  "account.oauth.codeSubmitted":
    "確認コードを送信しました。検証が完了するまで、このウィンドウを開いたままにしてください。",
  "account.oauth.validationCode":
    "ブラウザーに表示された英数字、アンダースコア、ハイフンのみの 32 文字の確認コードを入力してください。",

  "account.actions.clearLogin": "ログアウト / ローカルのログインを解除",
  "account.title": "アカウント",
  "account.description":
    "公式 Termeta アカウントにはメールとパスワード、GitHub、Google でログインできます。ローカル端末はアカウントなしで使用できます。",
  "account.encryptionHint":
    "ログインしても保管庫は解除されません。保管庫のパスワードやバックアップの暗号化パスワードは引き続き必要です。",
  "account.admin": "管理者",
  "account.createdAt": "登録日時：{date}",
  "account.expiresAt": "セッション有効期限：{date}",
  "account.signedOut": "ログインしていません。",
  "account.actions.refresh": "アカウントを更新",
  "account.actions.logout": "ログアウト",
  "account.actions.login": "ログイン",
  "account.actions.register": "登録",
  "account.actions.retry": "アカウントを再読込",
  "account.actions.unlink": "連携を解除",
  "account.actions.linkProvider": "{provider} を連携",
  "account.actions.loginProvider": "{provider} でログイン",
  "account.actions.retryProviders": "ログイン方法を再読込",
  "account.actions.cancelLogin": "待機をキャンセル",
  "account.actions.openSettings": "アカウントと設定",
  "account.identities.title": "ログイン方法",
  "account.identities.description":
    "連携前にブラウザーで外部アカウントを確認してください。メールが同じでもアカウントは自動統合されません。",
  "account.identities.password": "メールとパスワード",
  "account.identities.lastMethod":
    "ログイン方法を少なくとも 1 つ残してください。",
  "account.unlink.confirm":
    "{provider} との連携を解除しますか？解除後、そのアカウントでは Termeta にログインできなくなります。",
  "account.logout.confirm":
    "公式アカウントからログアウトしますか？ローカルの保管庫と端末セッションは引き続き使用できます。",
  "account.oauth.completed": "認証が完了し、アカウント情報が更新されました。",
  "account.oauth.cancelled": "ブラウザー認証をキャンセルしました。",
  "account.oauth.waiting":
    "システムブラウザーで {provider} の認証を完了してください。その後、自動的に Termeta に戻って検証を完了します。",
  "account.oauth.deadline": "待機期限：{date}",
  "account.errors.expired":
    "認証の有効期限が切れました。もう一度開始してください。",
  "account.errors.denied":
    "認証が完了しませんでした。再試行するか別の方法でログインしてください。",
  "account.form.email": "メールアドレス",
  "account.form.password": "パスワード",
  "account.form.confirmation": "パスワードの確認",
  "account.validation.email": "有効なメールアドレスを入力してください。",
  "account.validation.password":
    "パスワードは 12〜256 文字で入力してください。",
  "account.validation.confirmation": "パスワードが一致しません。",
} as const;

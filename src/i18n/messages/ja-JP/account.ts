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
  "account.title": "アカウント",
  "account.description":
    "公式アカウントモードでは保管庫全体を termeta.io に自動同期します。ローカルモードにアカウントは不要です。",
  "account.actions.logout": "ログアウト",
  "account.actions.loginProvider": "{provider} でログイン",
  "account.actions.retryProviders": "ログイン方法を再読込",
  "account.actions.cancelLogin": "待機をキャンセル",
  "account.actions.openSettings": "アカウントと設定",
  "account.logout.confirm":
    "ログアウトしてローカルモードに切り替えますか？ローカルデータとターミナルセッションは保持され、公式の自動同期は停止します。",
  "account.oauth.waiting":
    "システムブラウザーで {provider} の認証を完了してください。その後、自動的に Termeta に戻って検証を完了します。",
  "account.oauth.deadline": "待機期限：{date}",
  "account.errors.expired":
    "認証の有効期限が切れました。もう一度開始してください。",
  "account.errors.denied":
    "認証が完了しませんでした。再試行するか別の方法でログインしてください。",
  "account.mode.title": "Termeta の利用方法",
  "account.mode.description":
    "公式アカウントでログインして保管庫を termeta.io に自動同期するか、ローカルのオフラインモードでデータを管理します。",
  "account.mode.official": "公式アカウントモード",
  "account.mode.local": "ローカル / オフラインモード",
  "account.mode.logoutLocal": "ログアウトしてローカルモードを使用",
  "account.browser.description":
    "アカウント登録、ログイン、ログイン方法の管理は、ブラウザーの termeta.io で行います。",
  "account.sync.unlockTitle": "クラウド保管庫を解除",
  "account.sync.unlockDescription":
    "クラウドにデータがあります。保管庫のパスワードを入力し、検証後にローカルデータを統合して続行します。パスワードが誤っている場合は入れません。ログアウトするとローカルデータを保持してローカルモードを利用できます。",
  "account.sync.setupDescription":
    "保管庫のパスワードを設定してください。ローカルと公式クラウドのデータを同じパスワードで保護します。忘れた保管庫パスワードはアカウントへのログインでは復元できません。",
  "account.sync.existingDescription":
    "現在のローカル保管庫のパスワードを入力して公式自動同期を有効にします。公式クラウドとローカル保管庫は同じパスワードを使用します。",
  "account.sync.description":
    "公式クラウドへの全体自動同期が有効です。ローカル保管庫と同じマスターキーを使用します。ログアウトするとローカルデータを保持し、公式同期を停止します。",
  "account.sync.mainWindowRequired":
    "Termeta のメインウィンドウに戻り、保管庫パスワードを入力してクラウドデータを復号・統合してください。検証が完了するまでワークベンチには入れません。",
  "account.sync.lastSynced": "最終同期：{date}",
  "account.sync.now": "今すぐ同期",
  "account.sync.continue": "検証して続行",
  "account.sync.localUnlockTitle": "先にローカル保管庫を解除",
  "account.sync.localUnlockDescription":
    "ローカルデータを保持して統合するため、現在のローカル保管庫のパスワードを入力してください。別の端末でクラウドのパスワードを変更した場合、ローカルの古いパスワードと異なることがあります。ログアウトせず、ローカルの解除後にクラウドのパスワードを検証します。",
} as const;

// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "backup.title": "バックアップと同期",
  "backup.form.provider": "バックアップサービス",
  "backup.form.deviceName": "デバイス名",
  "backup.form.devicePlaceholder": "例：自分のMacBook",
  "backup.form.webdavUrl": "WebDAVフォルダURL（HTTPS）",
  "backup.form.password": "バックアップのパスワード（12文字以上）",
  "backup.form.confirmPassword": "バックアップのパスワードを確認",
  "backup.form.operation": "バックアップまたは同期",
  "backup.empty": "このフォルダまたはアカウントにバックアップがありません。",
  "backup.import.title": "クラウドからのインポートを確認",
  "backup.description":
    "ホスト、グループ、認証情報、転送ルール、信頼済みホスト鍵を暗号化してバックアップします。アプリ設定はこのデバイスに残ります。バックアップのパスワードを安全に保管してください。",
  "backup.passwordHint":
    "バックアップのパスワードはこのデバイスに保存されます。変更は新しいバックアップにのみ適用され、古いバックアップには元のパスワードが必要です。",
  "backup.actions.test": "接続をテスト",
  "backup.actions.saveConfig": "クラウド設定を保存",
  "backup.actions.uploadVersion":
    "バックアップ · 新しいバージョンをアップロード",
  "backup.actions.downloadVersion": "同期 · クラウドからダウンロード",
  "backup.actions.preview": "ダウンロードしてプレビュー",
  "backup.actions.loadMore": "バックアップをさらに読み込む",
  "backup.form.githubToken": "GitHubトークン（Gist権限が必要）",
  "backup.form.webdavPassword": "WebDAVパスワード / アプリパスワード",
  "backup.form.savedSecretPlaceholder":
    "保存済み。空欄で保持、新しい値で置き換え",
  "backup.form.credentialsHint": "認証情報はこのデバイスで暗号化されます",
  "backup.form.passwordHint":
    "バックアップのパスワードはこのデバイスで暗号化されます",
  "backup.version.undecryptable": "バックアップを復号できません",
  "backup.version.decryptError":
    "パスワードが違うか、バックアップが破損しています。",
  "backup.upload.description":
    "アップロードのたびに新しいバージョンのバックアップを作成します。",
  "backup.download.description":
    "バージョンを選び、プレビューしてからインポートを確認してください。",
  "backup.upload.progress": "バックアップを暗号化してアップロード",
  "backup.download.progress": "クラウドバックアップを読み込む",
  "backup.import.mergeDescription":
    "既存のローカルデータと設定を保持して、データを追加します。繰り返すと重複が作成されます。競合するホスト鍵は警告してスキップします。",
  "backup.actions.confirmImport": "新規データとしてインポート",
  "backup.import.counts":
    "ホスト {hosts} · グループ {groups} · 認証情報 {credentials} · 転送 {forwards} · 信頼済み鍵 {knownHosts} · スニペット {snippets}",
  "backup.import.success":
    "同期完了：{counts}。既存のローカルデータと設定は保持されました。",
  "backup.defaultDeviceName": "自分のデバイス",
  "backup.validation.deviceName":
    "デバイス名を入力してください（256バイト以内）",
  "backup.validation.urlFormat":
    "認証情報、クエリパラメーター、フラグメントを含まないHTTPSフォルダURLを入力してください",
  "backup.validation.webdavUrl":
    "有効なWebDAVのHTTPSフォルダURLを入力してください",
  "backup.validation.username":
    "コロンを含まないユーザー名を入力してください（1,024バイト以内）",
  "backup.validation.credentials": "クラウドの認証情報を入力してください",
  "backup.validation.passwordRequired":
    "バックアップのパスワードを入力してください",
  "backup.validation.passwordMaxLength":
    "パスワードは1,024バイト以内にしてください",
  "backup.validation.passwordMinLength":
    "バックアップのパスワードは12文字以上にしてください",
  "backup.validation.passwordMismatch":
    "バックアップのパスワードが一致しません",
  "backup.status.configSaved":
    "クラウド設定を暗号化し、このデバイスに保存しました。",
  "backup.status.connected": "接続に成功しました。",
  "backup.status.uploaded":
    "新しいバックアップをアップロードしました。既存のクラウドバックアップは保持されました。",
} as const;

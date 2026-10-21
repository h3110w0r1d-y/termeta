// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "credentials.validation.passwordRequired": "パスワードを入力してください",
  "credentials.confirmDelete":
    "保管庫から認証情報「{label}」を削除しますか？この操作は取り消せません。",
  "credentials.description":
    "パスワードや秘密鍵を保存し、複数のホストで利用できます。",
  "credentials.actions.new": "新しい認証情報",
  "credentials.search.label": "認証情報を検索",
  "credentials.search.placeholder": "認証情報の名前を検索…",
  "credentials.status.loading": "認証情報を読み込み中…",
  "credentials.search.noResults":
    "一致する認証情報がありません。検索条件を変えてください。",
  "credentials.empty":
    "保存済みの認証情報はありません。ホストへの接続に使う認証情報を作成してください。",
  "credentials.actions.create": "認証情報を作成",
  "credentials.types.password": "パスワード",
  "credentials.types.privateKey": "秘密鍵",
  "credentials.status.ready": "準備完了",
  "credentials.status.awaitingRecovery": "復旧待ち",
  "credentials.status.pendingDeletion": "削除待ち",
  "credentials.delete.referenceHint":
    "先にホスト設定からこの認証情報の参照を解除してください",
  "credentials.actions.delete": "認証情報を削除",
  "credentials.actions.retryDelete": "削除を再試行",
  "credentials.form.description":
    "ホストの認証用として保管庫に安全に保存されます。",
  "credentials.form.namePlaceholder": "例：本番SSHパスワード",
  "credentials.form.sshPassword": "SSHパスワード",
  "credentials.form.sshPrivateKey": "SSH秘密鍵",
  "credentials.form.privateKeyPlaceholder":
    "BEGIN 行と END 行を含む OpenSSH 秘密鍵全体を貼り付けてください",
  "credentials.form.passphrase": "鍵のパスフレーズ（任意）",
  "credentials.form.passphraseHint":
    "空欄の場合、必要に応じて接続時に入力を求めます",
  "credentials.actions.save": "認証情報を保存",
  "credentials.delete.description":
    "削除する前にホスト設定から認証情報の参照を解除してください。",
  "credentials.form.keyHint":
    "秘密鍵は暗号化して保管庫に保存されます。上限は 1 MiB です。",
  "credentials.referenceCount": "{count}件のホストで使用中",
  "credentials.actions.showPublicKey": "公開鍵を表示",
  "credentials.actions.copyPublicKey": "公開鍵をコピー",
  "credentials.publicKey.title": "SSH 公開鍵",
  "credentials.publicKey.loading": "公開鍵を読み込み中…",
  "credentials.publicKey.hint":
    "サーバーの authorized_keys ファイルに追加できます。",
  "credentials.publicKey.copied": "公開鍵をコピーしました",
} as const;

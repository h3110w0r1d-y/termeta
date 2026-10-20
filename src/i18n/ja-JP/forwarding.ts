// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "forwarding.validation.nameLength": "名前は128文字以内にしてください",
  "forwarding.form.bindAddress": "待ち受けIPアドレス",
  "forwarding.form.bindPort": "待ち受けポート",
  "forwarding.form.targetAddress": "接続先アドレス",
  "forwarding.form.targetPort": "接続先ポート",
  "forwarding.actions.edit": "ポート転送を編集",
  "forwarding.actions.new": "新しいポート転送",
  "forwarding.form.namePlaceholder": "例：データベース、開発サービス、プロキシ",
  "forwarding.form.hostHint": "先にホストページでSSHホストを作成してください。",
  "forwarding.form.mode": "転送モード",
  "forwarding.modes.local": "ローカル転送",
  "forwarding.modes.remote": "リモート転送",
  "forwarding.modes.dynamic": "動的SOCKS5",
  "forwarding.form.remoteBindAddress": "リモート待ち受けIP",
  "forwarding.form.localBindAddress": "ローカル待ち受けIP",
  "forwarding.form.loopbackPlaceholder": "127.0.0.1 または ::1",
  "forwarding.form.localTargetAddress": "ローカル接続先アドレス",
  "forwarding.form.remoteTargetAddress": "リモート接続先アドレス",
  "forwarding.form.targetPlaceholder":
    "ホスト名またはIP。IPv6に角括弧は不要です",
  "forwarding.confirmDelete":
    "ポート転送「{name}」を削除しますか？SSHホストは保持されます。",
  "forwarding.actions.create": "新しい転送",
  "forwarding.search.placeholder": "名前、SSHホスト、転送アドレスを検索",
  "forwarding.search.label": "ポート転送を検索",
  "forwarding.search.noResults": "一致する転送ルールがありません",
  "forwarding.empty": "転送ルールはありません",
  "forwarding.status.stopped": "停止中",
  "forwarding.errors.hostDeleted": "SSHホストは削除されました",
  "forwarding.endpoints.local": "ローカル",
  "forwarding.endpoints.dynamicTarget":
    "SOCKS5 → アプリケーションが接続先を指定",
  "forwarding.endpoints.localSide": "ローカル",
  "forwarding.endpoints.remoteSide": "リモート",
  "forwarding.endpoints.target": "→ {value1} {value2}",
  "forwarding.descriptions.local":
    "ローカルポートからリモートの接続先にアクセスします。接続先アドレスはSSHサーバーが名前解決します。",
  "forwarding.descriptions.remote":
    "サーバーのポートからローカル側の接続先にアクセスします。接続先アドレスはこのコンピューターが名前解決します。",
  "forwarding.descriptions.dynamic":
    "このコンピューターのみで利用できるSOCKS5プロキシを作成します。ホスト名はSSHサーバーが名前解決します。TCPのみ、認証なしです。",
  "forwarding.warning.publicListener":
    "0.0.0.0または::で待ち受けると、ほかのデバイスから接続できます。リモートアクセスはサーバー設定にも依存します。",
} as const;

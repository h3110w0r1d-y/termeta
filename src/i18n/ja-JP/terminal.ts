// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "terminal.punctuation.toggle": "中国語の記号変換を一時的に切り替え",
  "terminal.punctuation.on": "記号変換：オン",
  "terminal.punctuation.off": "記号変換：オフ",
  "terminal.punctuation.enable":
    "現在のターミナルの中国語記号変換を一時的に有効にします。ターミナルを閉じると保存済みの設定に戻ります。",
  "terminal.punctuation.disable":
    "現在のターミナルの中国語記号変換を一時的に無効にします。ターミナルを閉じると保存済みの設定に戻ります。",
  "terminal.regions.area": "ターミナル領域",
  "terminal.search.label": "ターミナル履歴を検索",
  "terminal.search.placeholder": "ターミナル履歴を検索…",
  "terminal.search.close": "検索を閉じる",
  "terminal.actions.copySelection": "選択範囲をコピー",
  "terminal.actions.paste": "貼り付け",
  "terminal.actions.find": "ターミナル内を検索",
  "terminal.actions.scrollToBottom": "一番下へスクロール",
  "terminal.actions.openLink": "リンクを開く",
  "terminal.titles.local": "ローカルターミナル",
  "terminal.snapshot.header": "\r\n[切断前の全画面スナップショット]\r\n",
  "terminal.auth.title": "SSH認証",
  "terminal.snapshot.truncated":
    "[全画面スナップショットを1 MiBで切り詰めました]",
  "terminal.errors.unsentInput": "{error}；未送信の入力は自動再送されません。",
  "terminal.actions.closeNamed": "ターミナル {name} を閉じる",
  "terminal.errors.fontLoad":
    "ターミナルフォントを読み込めませんでした。設定を再適用するか、アプリを再起動してください。",
  "terminal.state.created": "接続準備完了",
  "terminal.paste.tooLarge": "1回の貼り付けは64 KiB以内にしてください。",
  "terminal.actions.cancelConnection": "接続をキャンセル",
  "terminal.state.startingLocal": "ローカルターミナルを起動中",
  "terminal.titles.command": "コマンド",
  "terminal.state.commandFinished":
    "コマンドが終了しました。再実行するにはコンテキストメニューを使用してください。",
  "terminal.auth.answerTooLong": "回答は16,384文字以内にしてください",
  "terminal.paste.broadcastDescription":
    "{count}個のターミナルに{lines}行を貼り付け",
  "terminal.paste.currentDescription": "このターミナルに{lines}行を貼り付け",
  "terminal.state.closed": "接続を閉じました",
  "terminal.state.disconnected": "切断済み",
  "terminal.state.connected": "接続済み",
  "terminal.state.startingSsh": "SSH接続を開始中",
  "terminal.state.startingTelnet": "Telnet接続を開始中（暗号化なし）",
  "terminal.state.opening": "ターミナルを開いています",
  "terminal.paste.warning":
    "改行によりリモートコマンドが即座に実行される場合があります。続ける前に内容を確認してください。",
  "terminal.search.history": "ターミナル履歴を検索",
  "terminal.actions.disconnect": "切断",
  "terminal.errors.outputRead":
    "ターミナル出力を読み取れません。再接続してください。",
  "terminal.errors.sessionClosing":
    "前の接続を終了しています。しばらくしてから再試行してください。",
  "terminal.errors.inputUnavailable":
    "入力を送信できません。再接続してください。",
  "terminal.state.closing": "切断中",
  "terminal.state.unavailable": "接続中または接続終了",
  "terminal.paste.confirmMultiline": "複数行の貼り付けを確認",
  "terminal.paste.confirmBroadcast": "{targetCount}個のターミナルに一斉送信",
  "terminal.paste.confirmCurrent": "このターミナルに貼り付け",
  "terminal.state.awaitingHostKey": "ホストの確認待ち",
  "terminal.state.awaitingAuth": "認証待ち",
  "terminal.titles.terminal": "ターミナル",
  "terminal.errors.outputPending":
    "出力を処理しています。しばらくしてから再試行してください。履歴は保持されています。",
  "terminal.errors.outputFailure":
    "ターミナル出力が中断されました。再接続してください。",
  "terminal.errors.tcpConnectFailed":
    "指定されたホストとポートに接続できません。",
  "terminal.form.theme": "ターミナル配色",
  "terminal.accessibility.title": "ターミナル：{name}",
  "terminal.state.authenticating": "認証中",
  "terminal.theme.hostDefault": "ホスト / 既定の配色を使用",
  "terminal.paste.queueFull":
    "入力が混み合っています。少し待ってから貼り付けてください。",
  "terminal.state.connecting": "接続中",
  "terminal.state.failed": "接続失敗",
  "terminal.state.ended": "接続終了",
  "terminal.auth.target": "接続先：{targetTitle}",
  "terminal.state.incompleteOutput":
    "接続が終了しました。末尾の出力が欠けている可能性があります。",
  "terminal.actions.reopenLocal":
    "ローカルターミナルを開き直しています。履歴は保持されます。",
  "terminal.actions.rerun": "再実行",
  "terminal.actions.reconnect": "再接続",
  "terminal.actions.reconnectWithHistory":
    "再接続しています。ターミナル履歴は保持されます。",
  "terminal.exit.signal": " （シグナル {exitSignal}）",
  "terminal.exit.code": " （終了コード {exitCode}）",
} as const;

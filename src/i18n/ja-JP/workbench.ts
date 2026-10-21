// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "workbench.sidebar.themes": "テーマ",
  "workbench.sidebar.toggle": "ツールサイドバーを切り替え",
  "workbench.sidebar.label": "ターミナルツール",
  "workbench.sidebar.themeHint":
    "フォントとサイズはすべてのターミナルに適用されます。配色を選ぶと現在のホストを既定に戻し、全体の配色を更新します。",
  "workbench.sidebar.noTerminal":
    "スニペットを使うには接続中のターミナルを選択してください。",
  "workbench.sidebar.execute": "実行",

  "workbench.actions.newTab": "新しいタブ",
  "workbench.actions.newSshConnection": "新しいSSH接続",
  "workbench.actions.closeCurrentTab": "現在のタブを閉じる",
  "workbench.actions.nextTab": "次のタブ",
  "workbench.actions.previousTab": "前のタブ",
  "workbench.actions.newLocalTerminal": "新しいローカルターミナル",
  "workbench.terminalCount": "{count}個のターミナル",
  "workbench.close.confirmGroup":
    "「{title}」の{count}個のターミナルでコマンドが実行中の可能性があります。閉じると接続を切断し、セッションログを保存します。続けますか？",
  "workbench.close.confirmTerminal":
    "「{title}」でコマンドが実行中の可能性があります。閉じると接続を切断し、セッションログを保存します。続けますか？",
  "workbench.layout.swapVertical": "上下を入れ替え",
  "workbench.layout.swapHorizontal": "左右を入れ替え",
  "workbench.actions.closeNamed": "{name}を閉じる",
  "workbench.broadcast.disable":
    "一斉送信を停止（{broadcastCount}個のターミナル）",
  "workbench.broadcast.commandExcluded":
    "コマンド用ターミナルは一斉入力を受信しません",
  "workbench.actions.duplicateTab": "タブを複製",
  "workbench.tabs.label": "アプリケーションのタブ",
  "workbench.broadcast.enable":
    "{broadcastCount}個のターミナルに入力を一斉送信",
  "workbench.broadcast.unsupported":
    "このターミナルは入力の一斉送信に対応していません。",
  "workbench.actions.split": "セッションを分割",
  "workbench.actions.newTabWithShortcut": "新しいタブ（Cmd/Ctrl+T）",
  "workbench.broadcast.minimumTerminals":
    "先に2つ以上の対話型ターミナルを接続してください",
  "workbench.layout.resizeVertical": "上下のターミナルのサイズを変更",
  "workbench.layout.resizeHorizontal": "左右のターミナルのサイズを変更",
} as const;

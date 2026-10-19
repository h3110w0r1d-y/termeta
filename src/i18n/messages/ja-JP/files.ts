// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "files.sidebar.toggle": "SFTPの表示を切り替え",
  "files.errors.sshDisconnected":
    "SSHが切断されました。再接続後に更新してください。",
  "files.count": "{count}件",
  "files.countPartial": "{count}件（続きあり）",
  "files.transfer.summary":
    "{name} · {completed}/{total}ファイル · {transferred}{size}",
  "files.actions.upload": "アップロード",
  "files.actions.parent": "親フォルダ",
  "files.actions.download": "ダウンロード",
  "files.actions.home": "ホームフォルダ",
  "files.transfer.active": "転送中",
  "files.transfer.progress": "転送中 {completed}/{total}",
  "files.transfer.complete": "転送完了",
  "files.transfer.queue": "転送キュー",
  "files.types.other": "その他のファイル",
  "files.copy.preparing": "コピーを準備中…",
  "files.errors.staleList":
    "ファイル一覧が古くなっています。更新してください。",
  "files.actions.go": "移動",
  "files.actions.loadMorePage": "さらに読み込む（100件）",
  "files.browser.title": "2ペインSFTPブラウザー",
  "files.actions.cancelTransfer": "転送をキャンセル",
  "files.actions.cancelCopy": "コピーをキャンセル",
  "files.copy.unsupportedType":
    "通常のファイルとフォルダのみコピーできます。シンボリックリンクは未対応です。",
  "files.copy.dragHint": "{count}件をコピー · もう一方のペインにドラッグ",
  "files.copy.confirm": "{count}件をもう一方のフォルダにコピー",
  "files.actions.copyToOtherPane": "もう一方のペインにコピー",
  "files.fields.size": "サイズ",
  "files.copy.complete": "コピー完了",
  "files.upload.confirmDrop":
    "ドロップしたファイルやフォルダを{target}にアップロードしますか？同名フォルダは統合されます。既存ファイルの上書きはその都度確認します。",
  "files.transfer.bytes": "転送済み {transferred} / {total}",
  "files.selection.loaded": "{loaded}件読み込み済み · {selected}件選択中",
  "files.transfer.removed": "キューから削除済み",
  "files.selection.count": "{count}件選択中",
  "files.selection.named": "選択中：{name}",
  "files.transfer.queueFull":
    "キューが上限の32件に達しました。転送が完了するまでお待ちください。",
  "files.types.file": "ファイル",
  "files.types.directory": "フォルダ",
  "files.types.directoryLink": "フォルダへのシンボリックリンク",
  "files.types.fileLink": "ファイルへのシンボリックリンク",
  "files.errors.tooManyEntries":
    "ファイルが多すぎて表示できません。サブフォルダを開いてください。",
  "files.confirmDisconnect":
    "SSHを切断すると、進行中のSFTP転送がキャンセルされます。続けますか？",
  "files.actions.showHidden": "隠しファイルを表示",
  "files.fields.permissions": "アクセス権",
  "files.upload.dropHint": "ドロップして{path}にアップロード",
  "files.transfer.inProgress": "転送中",
  "files.confirmCloseTerminal":
    "SFTP転送が進行中です。閉じるとキャンセルされます。続けますか？",
  "files.sidebar.label": "このターミナルのSFTPブラウザー",
  "files.errors.dropBusy":
    "現在の操作が終了してから、もう一度ファイルをドラッグしてください。",
  "files.path.placeholder": "リモートパスを入力",
  "files.sidebar.resize": "SFTPサイドバーのサイズを変更",
  "files.types.unknownLink": "シンボリックリンク（リンク先の種類不明）",
  "files.path.label": "リモートパス",
  "files.transfer.connectionChanged":
    "接続が変更されました。転送は開始されませんでした。",
  "files.hostPlaceholder": "ファイルを参照するホストを選択",
  "files.transfer.preparing": "ファイルの選択または転送の準備中…",
  "files.sidebar.hide": "SFTPサイドバーを非表示",
  "files.transfer.hidePanel": "パネルを非表示にして転送を継続",
} as const;

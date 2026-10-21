// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "files.hosts.search": "SSH ホストを検索…",
  "files.hosts.empty":
    "保存済みの SSH ホストがありません。先に保管庫で追加してください。",
  "files.drag.pastePaths": "放して {count} 個のパスを貼り付け",
  "files.drag.hint":
    "{count} 項目 · ターミナルにドロップしてパスを貼り付け、ウィンドウ外にドラッグしてダウンロード",
  "files.drag.invalidPaths":
    "選択したパスが長すぎるか、未対応の文字が含まれています。選択項目を減らして再試行してください。",
  "files.follow.label": "ターミナルの作業ディレクトリに追従",
  "files.follow.description":
    "ターミナルがディレクトリの変更を通知すると SFTP を同期します。ファイル操作の終了後、最新のディレクトリに移動します。",
  "files.follow.waiting":
    "作業ディレクトリの通知を待っています。シェルによる通知、またはタイトルへのフルパス表示が必要です。",
  "files.navigation.collapsed": "省略された上位フォルダーを表示",
  "files.navigation.edit": "フォルダーのパスを編集",
  "files.activity.title": "転送履歴",
  "files.activity.open": "転送一覧を表示",
  "files.activity.active": "{count} 件を転送中",
  "files.activity.clearFinished": "終了した転送を消去",
  "files.activity.empty": "転送はまだありません",
  "files.activity.idle": "すべての転送が終了しました",
  "files.activity.failed": "転送失敗",
  "files.activity.cancelling": "キャンセル中…",
  "files.activity.progress": "転送の進行状況",
  "files.activity.unknownProgress": "準備中です。全体の進行状況はまだ不明です",
  "files.table.label": "リモートファイル一覧",
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
  "files.manage.delete.action": "削除",
  "files.manage.delete.title": "選択した項目を削除",
  "files.manage.delete.recursiveTitle": "フォルダーと内容の削除を確認",
  "files.manage.delete.warning":
    "これらの項目をリモートから完全に削除します。元に戻せません。リンクはリンク自体のみ削除します。",
  "files.manage.delete.recursiveWarning":
    "選択したフォルダーと、隠しファイルを含むすべての内容を完全に削除します。中断や失敗が発生しても、削除済みの項目は復元できません。再帰的に削除しますか？",
  "files.manage.delete.continue": "フォルダーの確認へ進む",
  "files.manage.delete.permanent": "完全に削除",
  "files.manage.selection": "{count} 項目を選択中",
  "files.manage.partialFailure":
    "一部の変更は完了している可能性があります。更新された一覧を確認してから再試行してください。完了した変更は元に戻りません。",
  "files.manage.limit": "一度に操作できるのは 100 項目までです。",
  "files.manage.permissions.title": "アクセス権を編集",
  "files.manage.permissions.applies":
    "選択したすべての項目に適用します。フォルダーの内容は再帰的に変更しません。現在の権限が異なる場合や不明な場合は値を指定してください。",
  "files.manage.permissions.owner": "所有者",
  "files.manage.permissions.group": "グループ",
  "files.manage.permissions.others": "その他",
  "files.manage.permissions.read": "読み取り",
  "files.manage.permissions.write": "書き込み",
  "files.manage.permissions.execute": "実行 / 移動",
  "files.manage.permissions.code": "8 進数のアクセス権",
  "files.manage.permissions.invalid":
    "3 桁または 4 桁の 8 進数（000〜7777）を入力してください。",
  "files.manage.permissions.special": "特殊な権限（詳細）",
  "files.manage.permissions.setuid": "Setuid：所有者として実行",
  "files.manage.permissions.setgid":
    "Setgid：グループを継承 / グループとして実行",
  "files.manage.permissions.sticky":
    "Sticky：ディレクトリ内の項目の削除を所有者に制限",
  "files.manage.delete.confirmContents":
    "選択したフォルダーとすべての内容が完全に削除されることを理解しました。",
} as const;

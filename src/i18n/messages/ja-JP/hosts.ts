// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "hosts.batch.title": "複数のホストに接続",
  "hosts.batch.confirm":
    "サブグループを含め、「{groupName}」内の全{count}ホストに接続しますか？",
  "hosts.batch.separateTabs": "個別のタブで開く",
  "hosts.batch.workspace": "ワークスペースで開く",
  "hosts.auth.password": "パスワード",
  "hosts.auth.publicKey": "公開鍵",
  "hosts.startup.shell": "対話型シェル",
  "hosts.startup.command": "コマンドを実行",
  "hosts.validation.privateKeyRequired": "秘密鍵を選択してください",
  "hosts.groups.ungrouped": "未分類",
  "hosts.form.address": "ホストアドレス",
  "hosts.form.icon": "アイコン",
  "hosts.form.startupMode": "起動モード",
  "hosts.form.startupCommand": "起動コマンド",
  "hosts.form.authentication": "認証",
  "hosts.form.privateKeyCredential": "秘密鍵の認証情報",
  "hosts.form.privateKeyHint":
    "秘密鍵の認証情報を選択するか、「新しい認証情報」で作成してください",
  "hosts.actions.quickConnect": "クイック接続",
  "hosts.actions.newCredentialOption": "+ 新しい認証情報",
  "hosts.actions.edit": "ホストを編集",
  "hosts.actions.new": "新しいホスト",
  "hosts.form.namePlaceholder": "例：本番 · 東京",
  "hosts.form.iconLabel": "ホストアイコン",
  "hosts.form.passwordCredential": "パスワードの認証情報",
  "hosts.form.password": "ホストのパスワード（任意）",
  "hosts.form.passwordHint":
    "このホストとともに安全に保存します。空欄の場合は接続時に入力を求めます",
  "hosts.form.commandPlaceholder": "例：/usr/bin/btop または docker stats",
  "hosts.form.theme": "ホストの配色",
  "hosts.form.tagsPlaceholder": "本番, Linux",
  "hosts.actions.retryTheme": "配色の保存を再試行",
  "hosts.actions.save": "ホストを保存",
  "hosts.errors.sidebarSave":
    "サイドバーの設定を保存できませんでした：{value1}",
  "hosts.errors.expansionSave":
    "グループの展開状態を保存できませんでした：{value1}",
  "hosts.filters.all": "すべてのホスト",
  "hosts.groups.topLevel": "最上位",
  "hosts.groups.newChild": "新しいサブグループ",
  "hosts.groups.create": "グループを作成",
  "hosts.groups.editParent": "グループの編集 / 親の変更",
  "hosts.groups.deleteMenu": "グループを削除…",
  "hosts.groups.topLevelDestination": "最上位（ホストは未分類になります）",
  "hosts.groups.name": "グループ名",
  "hosts.groups.confirmDeleteAll":
    "「{name}」と、その配下の{groups}個のサブグループおよび{hosts}件のホストを完全に削除しますか？ホストのパスワードも削除されます。この操作は取り消せません。既存のセッションと共有認証情報は保持されます。",
  "hosts.groups.refreshAfterDeleteFailed":
    "グループは削除されましたが、一覧を更新できませんでした：{value1}。一覧を更新してください。",
  "hosts.groups.title": "ホストグループ",
  "hosts.groups.new": "新しいグループ",
  "hosts.groups.showHosts": "ホストを表示",
  "hosts.groups.showHostsInSidebar": "サイドバーにホストを表示",
  "hosts.groups.edit": "グループを編集",
  "hosts.groups.parent": "親グループ",
  "hosts.groups.delete": "グループを削除",
  "hosts.groups.deletePrompt": "「{name}」の内容の扱いを選択してください：",
  "hosts.groups.keepContents": "ホストとサブグループを親に移動して保持",
  "hosts.groups.moveContentsDescription":
    "直下の{direct}件のホストとすべての直下のサブグループを{parent}へ移動します。サブグループ内の内容はそのまま保持されます。",
  "hosts.groups.deleteContents": "すべてのサブグループとホストを削除",
  "hosts.groups.deleteContentsDescription":
    "{groups}個のサブグループとその中の全{hosts}件のホストを削除します。もう一度確認が表示されます。",
  "hosts.groups.deleteAndMove": "グループを削除して内容を移動",
  "hosts.search.limit":
    "{count}件のホストのうち先頭100件を表示しています。検索条件を絞り込んでください。",
  "hosts.status.favorite": "お気に入り",
  "hosts.search.placeholder": "すべてのホストを検索…",
  "hosts.search.label": "すべてのホストを検索",
  "hosts.search.noResults": "一致するホストがありません",
  "hosts.search.hint": "入力してすべてのホストを検索",
  "hosts.actions.moveToGroup": "グループへ移動…",
  "hosts.validation.invalidGroup": "有効なグループを選択してください",
  "hosts.confirmDelete":
    "ホスト「{name}」を削除しますか？既存のセッションと共有認証情報は保持されます。",
  "hosts.filters.group": "グループ内のホスト",
  "hosts.count": "{count}件のホスト",
  "hosts.filters.includeSubgroups": "サブグループを含める",
  "hosts.view.label": "ホストの表示モード",
  "hosts.view.cards": "カード表示",
  "hosts.view.list": "リスト表示",
  "hosts.groups.empty":
    "このグループにホストはありません。作成するか、別のグループからドラッグしてください。",
  "hosts.actions.createHere": "ここにホストを作成",
  "hosts.actions.favorite": "お気に入りに追加",
  "hosts.actions.move": "ホストを移動",
  "hosts.move.prompt": "「{name}」の移動先：",
  "hosts.move.destination": "移動先グループ",
  "hosts.newTab.description":
    "保存済みホストを選択するか、クイック接続を使用してください。",
  "hosts.newTab.searchLabel": "新しいタブ用のホストを検索",
  "hosts.newTab.available": "利用可能なホスト",
  "hosts.newTab.empty":
    "保存済みホストはありません。クイック接続から始められます。",
  "hosts.search.limitDescription":
    "{count}件のホストのうち先頭100件を表示しています。検索条件を絞り込んでください。",
  "hosts.drag.destination": "移動先：{target}",
  "hosts.drag.invalidTarget": "ここにはドロップできません",
  "hosts.batch.description":
    "個別のタブまたは分割ワークスペースを選択してください。",
  "hosts.batch.savedWorkspaceHint":
    "保存済みワークスペースは元のレイアウトで開きます。",
  "hosts.form.useHostPassword": "ホスト専用のパスワードを使用",
  "hosts.errors.themeSave":
    "ホストは保存されましたが、配色は保存されませんでした。再試行してください：{value1}",
  "hosts.validation.invalidCommand":
    "コマンドが長すぎるか、無効な文字が含まれています",
  "hosts.validation.nameConflict":
    "この名前は別のホストまたはワークスペースで使用されています",
  "hosts.accessibility.connect":
    "{name}、Enterで接続、右クリックまたはShift+F10で操作を表示",
  "hosts.errors.savedWithoutTheme":
    "ホストは保存されましたが、配色を保存できませんでした。再試行してください。{error}",
  "hosts.selection.sshRequired": "保存済みSSHホストを選択してください",
  "hosts.selection.sshPlaceholder": "保存済みSSHホストを選択",
} as const;

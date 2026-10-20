// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "workspaces.title": "ワークスペース",
  "workspaces.errors.missingLayout":
    "このワークスペースには保存済みレイアウトがありません。",
  "workspaces.actions.save": "ワークスペースを保存",
  "workspaces.actions.saveMenu": "ワークスペースを保存…",
  "workspaces.save.description":
    "現在のレイアウトと接続を保存します。同じ名前で再度保存すると、このワークスペースを更新します。",
  "workspaces.errors.workspaceOnly": "ここではワークスペースのみ保存できます。",
  "workspaces.errors.nameConflict":
    "この名前は接続で使用されています。同名のワークスペースのみ置き換えられます。",
  "workspaces.save.confirmReplace":
    "ワークスペース「{name}」のレイアウトと接続を置き換えます。",
  "workspaces.errors.deletedConnection":
    "ワークスペース「{name}」で使用する接続が削除されました。ワークスペースを保存し直してください。",
  "workspaces.errors.singleConnection":
    "このワークスペースは保管庫から開いてください。",
  "workspaces.errors.invalidSplit": "ワークスペースのレイアウトが無効です。",
  "workspaces.errors.unsavedConnections":
    "ワークスペースを保存する前に、すべてのSSH、Telnet、シリアル接続を保存してください。ローカルターミナルはまだ保存できません。",
  "workspaces.errors.layoutLimit":
    "ワークスペースのレイアウトが対応サイズを超えています。",
  "workspaces.errors.invalidFocus":
    "選択されたワークスペースのペインが無効です。",
  "workspaces.errors.invalidConnection": "ワークスペース内の接続が無効です。",
  "workspaces.actions.replace": "ワークスペースを置き換え",
} as const;

// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "shortcuts.triggers.click": "クリック",
  "shortcuts.validation.conflict": "ショートカットの競合：{first}と{second}",
  "shortcuts.validation.invalidEntries": "ショートカットの項目が無効です。",
  "shortcuts.validation.invalidFields": "ショートカットの設定が無効です。",
  "shortcuts.validation.contextMismatch":
    "この状況ではショートカットを使用できません。",
  "shortcuts.validation.linkRequiresMouse":
    "リンクを開くにはマウスショートカットを使用してください。",
  "shortcuts.validation.unsupportedKey": "このキーは対応していません。",
  "shortcuts.validation.unsupportedTrigger":
    "この種類のショートカットは対応していません。",
  "shortcuts.import.confirm":
    "{count}件の割り当てを読み込み、現在のショートカットを置き換えますか？",
  "shortcuts.categories.label": "ショートカットのカテゴリ",
  "shortcuts.search.placeholder": "操作やキーを検索…",
  "shortcuts.actions.addBinding": "ショートカットを追加",
  "shortcuts.actions.resetCommand": "この操作をリセット",
  "shortcuts.actions.deleteBinding": "ショートカットを削除",
  "shortcuts.categories.application": "アプリケーションのショートカット",
  "shortcuts.categories.terminal": "ターミナルのショートカット",
  "shortcuts.warning.remoteKeys":
    "Ctrl+Cなどをアプリケーションのショートカットに割り当てると、そのキーはリモートホストに送信されなくなります。",
  "shortcuts.hint.nativeSettings":
    "これらの割り当てに関係なく、システムメニューのCmd+,で常に設定が開きます。設定を開く別のショートカットも登録できます。",
  "shortcuts.actions.resetAll": "すべてリセット",
  "shortcuts.actions.export": "設定をエクスポート",
  "shortcuts.actions.import": "設定をインポート",
  "shortcuts.validation.fileTooLarge": "ファイルが大きすぎます",
  "shortcuts.validation.unsupportedVersion":
    "未対応のショートカットファイルのバージョンです",
  "shortcuts.triggers.primaryClick": "Cmd / Ctrl + クリック",
  "shortcuts.triggers.altClick": "Alt + クリック",
  "shortcuts.triggers.shiftClick": "Shift + クリック",
  "shortcuts.recording.hint": "キーを押してください。Escでキャンセル",
  "shortcuts.recording.unbound": "未割り当て",
} as const;

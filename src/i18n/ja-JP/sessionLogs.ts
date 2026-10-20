// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "sessionLogs.showHistory": "スクロールバックを表示",
  "sessionLogs.showScreen": "終了時の画面を表示",
  "sessionLogs.invalidSnapshot":
    "ログの表示データが破損しているか未対応です。色を復元できません。",
  "sessionLogs.title": "セッションログ",
  "sessionLogs.description":
    "タブを閉じるかアプリを終了するとターミナルの内容を保存します。ログはこのデバイスにのみ保存され、バックアップには含まれません。",
  "sessionLogs.empty":
    "セッションログはありません。ターミナルのタブを閉じると、ここに内容が保存されます。",
  "sessionLogs.noMatch": "一致するセッションログがありません。",
  "sessionLogs.search": "名前、ユーザー名、ホスト、プロトコルで検索",
  "sessionLogs.actions.clear": "ログを消去",
  "sessionLogs.actions.open": "ログを表示",
  "sessionLogs.confirmClear":
    "ローカルのセッションログをすべて消去しますか？この操作は取り消せません。保存済みホストは保持されます。",
  "sessionLogs.confirmDelete":
    "「{name}」のセッションログを削除しますか？この操作は取り消せません。",
  "sessionLogs.tabTitle": "ログ · {name}",
  "sessionLogs.content": "セッションログの内容、読み取り専用",
  "sessionLogs.emptyContent":
    "このターミナルには終了時に保存できる内容がありませんでした。",
  "sessionLogs.timeRange": "開始 {started} · 保存 {closed}",
} as const;

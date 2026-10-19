// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "serial.errors.missingConfiguration": "シリアル設定がありません",
  "serial.connect.title": "シリアル接続",
  "serial.form.namePlaceholder": "例：開発ボード",
  "serial.form.stopBits": "ストップビット",
  "serial.parity.even": "偶数",
  "serial.actions.refresh": "シリアルポートを更新",
  "serial.parity.odd": "奇数",
  "serial.form.encoding": "文字コード",
  "serial.form.tagsPlaceholder": "ボード, デバッグ",
  "serial.actions.open": "シリアルポートを開いています",
  "serial.form.dataBits": "データビット",
  "serial.actions.newHost": "新しいシリアルホスト",
  "serial.empty":
    "シリアルポートが見つかりません。デバイスを接続して更新するか、名前を直接入力してください。Linuxではシリアルポートのアクセス権も確認してください。",
  "serial.form.parity": "パリティ",
  "serial.form.baudRate": "ボーレート",
  "serial.validation.baudRate": "ボーレートは1～4,000,000の整数にしてください",
  "serial.form.flowControl": "フロー制御",
  "serial.actions.editHost": "シリアルホストを編集",
  "serial.validation.device":
    "シリアルデバイスのパスまたは名前を、制御文字を含めず1,024バイト以内で入力してください",
  "serial.form.devicePlaceholder":
    "シリアルデバイスのパスまたは名前を選択または入力",
  "serial.form.baudRatePlaceholder": "ボーレートを選択または入力（既定値9600）",
  "serial.actions.reopen":
    "シリアルポートを開き直しています。ターミナル履歴は保持されます。",
  "serial.form.advanced": "詳細設定",
} as const;

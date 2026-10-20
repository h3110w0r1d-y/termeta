// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "snippets.description":
    "再利用するコマンドスニペットを元のテキストと改行のまま保存します。",
  "snippets.actions.new": "新しいスニペット",
  "snippets.actions.edit": "スニペットを編集",
  "snippets.actions.save": "スニペットを保存",
  "snippets.fields.content": "内容",
  "snippets.search.label": "スニペットを検索",
  "snippets.search.placeholder": "名前や内容を検索…",
  "snippets.search.noResults":
    "一致するスニペットがありません。検索条件を変えてください。",
  "snippets.empty": "保存済みのスニペットはありません。",
  "snippets.loading": "スニペットを読み込み中…",
  "snippets.confirmDelete":
    "スニペット「{name}」を削除しますか？この操作は取り消せません。",
  "snippets.form.description":
    "スニペットは暗号化された保管庫に保存され、クラウドバックアップに含まれます。",
  "snippets.form.namePlaceholder": "例：サービスの状態を確認",
  "snippets.form.contentPlaceholder":
    "複数行のコマンドスニペットを入力できます",
  "snippets.validation.contentLimit":
    "内容にNUL文字を含めることはできません。64 KiB以内で入力してください。",
} as const;

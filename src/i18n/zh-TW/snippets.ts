// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "snippets.description": "集中儲存常用命令片段，保留正文與換行。",
  "snippets.actions.new": "新建 Snippet",
  "snippets.actions.edit": "編輯 Snippet",
  "snippets.actions.save": "儲存 Snippet",
  "snippets.fields.content": "片段正文",
  "snippets.search.label": "搜尋 Snippets",
  "snippets.search.placeholder": "搜尋名稱或正文…",
  "snippets.search.noResults": "沒有匹配的片段，嘗試其他關鍵詞。",
  "snippets.empty": "還沒有儲存片段。",
  "snippets.loading": "正在讀取片段…",
  "snippets.confirmDelete": "刪除片段“{name}”？此操作不能撤銷。",
  "snippets.form.description": "片段儲存在加密保險庫中，並隨雲備份同步。",
  "snippets.form.namePlaceholder": "例如：檢視服務狀態",
  "snippets.form.contentPlaceholder": "輸入命令片段，可保留多行文字",
  "snippets.validation.contentLimit":
    "正文不能包含 NUL 字元，且最多為 64 KiB。",
} as const;

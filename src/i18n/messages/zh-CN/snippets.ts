// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "snippets.description": "集中保存常用命令片段，保留正文与换行。",
  "snippets.actions.new": "新建 Snippet",
  "snippets.actions.edit": "编辑 Snippet",
  "snippets.actions.save": "保存 Snippet",
  "snippets.fields.content": "片段正文",
  "snippets.search.label": "搜索 Snippets",
  "snippets.search.placeholder": "搜索名称或正文…",
  "snippets.search.noResults": "没有匹配的片段，尝试其他关键词。",
  "snippets.empty": "还没有保存片段。",
  "snippets.loading": "正在读取片段…",
  "snippets.confirmDelete": "删除片段“{name}”？此操作不能撤销。",
  "snippets.form.description": "片段保存在加密保险库中，并随云备份同步。",
  "snippets.form.namePlaceholder": "例如：查看服务状态",
  "snippets.form.contentPlaceholder": "输入命令片段，可保留多行文本",
  "snippets.validation.contentLimit":
    "正文不能包含 NUL 字符，且最多为 64 KiB。",
} as const;

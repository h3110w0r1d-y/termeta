// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "snippets.description":
    "Save reusable command snippets with their original text and line breaks.",
  "snippets.actions.new": "New snippet",
  "snippets.actions.edit": "Edit snippet",
  "snippets.actions.save": "Save snippet",
  "snippets.fields.content": "Content",
  "snippets.search.label": "Search snippets",
  "snippets.search.placeholder": "Search names or content…",
  "snippets.search.noResults": "No matching snippets. Try another search.",
  "snippets.empty": "No saved snippets yet.",
  "snippets.loading": "Loading snippets…",
  "snippets.confirmDelete": "Delete snippet “{name}”? This cannot be undone.",
  "snippets.form.description":
    "Snippets are saved in your encrypted vault and included in cloud backups.",
  "snippets.form.namePlaceholder": "e.g. Check service status",
  "snippets.form.contentPlaceholder":
    "Enter a command snippet, including multiple lines",
  "snippets.validation.contentLimit":
    "Content cannot contain NUL characters or exceed 64 KiB.",
} as const;

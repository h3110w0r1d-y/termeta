// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "snippets.description":
    "재사용할 명령 스니펫을 원문과 줄바꿈 그대로 저장합니다.",
  "snippets.actions.new": "새 스니펫",
  "snippets.actions.edit": "스니펫 편집",
  "snippets.actions.save": "스니펫 저장",
  "snippets.fields.content": "내용",
  "snippets.search.label": "스니펫 검색",
  "snippets.search.placeholder": "이름 또는 내용 검색…",
  "snippets.search.noResults":
    "일치하는 스니펫이 없습니다. 다른 검색어를 입력하세요.",
  "snippets.empty": "저장된 스니펫이 없습니다.",
  "snippets.loading": "스니펫 불러오는 중…",
  "snippets.confirmDelete":
    "스니펫 “{name}”을(를) 삭제할까요? 이 작업은 취소할 수 없습니다.",
  "snippets.form.description":
    "스니펫은 암호화된 보관함에 저장되며 클라우드 백업에 포함됩니다.",
  "snippets.form.namePlaceholder": "예: 서비스 상태 확인",
  "snippets.form.contentPlaceholder": "여러 줄의 명령 스니펫을 입력하세요",
  "snippets.validation.contentLimit":
    "내용에 NUL 문자를 포함하거나 64 KiB를 초과할 수 없습니다.",
} as const;

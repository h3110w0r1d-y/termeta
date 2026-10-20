// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "shortcuts.triggers.click": "클릭",
  "shortcuts.validation.conflict": "단축키 충돌: {first} 및 {second}",
  "shortcuts.validation.invalidEntries": "단축키 항목이 올바르지 않습니다.",
  "shortcuts.validation.invalidFields": "단축키 설정이 올바르지 않습니다.",
  "shortcuts.validation.contextMismatch":
    "현재 상황에서는 이 단축키를 사용할 수 없습니다.",
  "shortcuts.validation.linkRequiresMouse":
    "링크를 열려면 마우스 단축키를 사용하세요.",
  "shortcuts.validation.unsupportedKey": "지원하지 않는 키입니다.",
  "shortcuts.validation.unsupportedTrigger": "지원하지 않는 단축키 유형입니다.",
  "shortcuts.import.confirm":
    "단축키 {count}개를 가져와 현재 단축키를 교체할까요?",
  "shortcuts.categories.label": "단축키 범주",
  "shortcuts.search.placeholder": "작업 또는 키 검색…",
  "shortcuts.actions.addBinding": "단축키 추가",
  "shortcuts.actions.resetCommand": "이 작업 초기화",
  "shortcuts.actions.deleteBinding": "단축키 제거",
  "shortcuts.categories.application": "애플리케이션 단축키",
  "shortcuts.categories.terminal": "터미널 단축키",
  "shortcuts.warning.remoteKeys":
    "Ctrl+C 등을 애플리케이션 단축키로 지정하면 해당 키가 원격 호스트에 전송되지 않습니다.",
  "shortcuts.hint.nativeSettings":
    "이 설정과 관계없이 시스템 메뉴의 Cmd+,는 항상 설정을 엽니다. 설정을 여는 다른 단축키도 등록할 수 있습니다.",
  "shortcuts.actions.resetAll": "모두 초기화",
  "shortcuts.actions.export": "설정 내보내기",
  "shortcuts.actions.import": "설정 가져오기",
  "shortcuts.validation.fileTooLarge": "파일이 너무 큽니다",
  "shortcuts.validation.unsupportedVersion":
    "지원하지 않는 단축키 파일 버전입니다",
  "shortcuts.triggers.primaryClick": "Cmd / Ctrl + 클릭",
  "shortcuts.triggers.altClick": "Alt + 클릭",
  "shortcuts.triggers.shiftClick": "Shift + 클릭",
  "shortcuts.recording.hint": "키를 누르세요. Esc로 취소",
  "shortcuts.recording.unbound": "지정되지 않음",
} as const;

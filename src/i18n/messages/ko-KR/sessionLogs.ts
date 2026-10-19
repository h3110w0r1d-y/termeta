// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "sessionLogs.showHistory": "스크롤백 보기",
  "sessionLogs.showScreen": "종료 시 화면 보기",
  "sessionLogs.invalidSnapshot":
    "로그 표시 데이터가 손상되었거나 지원하지 않는 형식입니다. 색상을 복원할 수 없습니다.",
  "sessionLogs.title": "세션 로그",
  "sessionLogs.description":
    "탭을 닫거나 앱을 종료할 때 터미널 내용을 저장합니다. 로그는 이 기기에만 보관되며 백업에 포함되지 않습니다.",
  "sessionLogs.empty":
    "세션 로그가 없습니다. 터미널 탭을 닫으면 여기에 내용이 저장됩니다.",
  "sessionLogs.noMatch": "일치하는 세션 로그가 없습니다.",
  "sessionLogs.search": "이름, 사용자 이름, 호스트 또는 프로토콜로 검색",
  "sessionLogs.actions.clear": "로그 지우기",
  "sessionLogs.actions.open": "로그 보기",
  "sessionLogs.confirmClear":
    "모든 로컬 세션 로그를 지울까요? 이 작업은 취소할 수 없습니다. 저장된 호스트는 유지됩니다.",
  "sessionLogs.confirmDelete":
    "“{name}”의 세션 로그를 삭제할까요? 이 작업은 취소할 수 없습니다.",
  "sessionLogs.tabTitle": "로그 · {name}",
  "sessionLogs.content": "세션 로그 내용, 읽기 전용",
  "sessionLogs.emptyContent": "종료 시 이 터미널에 저장할 내용이 없었습니다.",
  "sessionLogs.timeRange": "시작 {started} · 저장 {closed}",
} as const;

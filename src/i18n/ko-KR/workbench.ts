// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "workbench.actions.newTab": "새 탭",
  "workbench.actions.newSshConnection": "새 SSH 연결",
  "workbench.actions.closeCurrentTab": "현재 탭 닫기",
  "workbench.actions.nextTab": "다음 탭",
  "workbench.actions.previousTab": "이전 탭",
  "workbench.actions.newLocalTerminal": "새 로컬 터미널",
  "workbench.terminalCount": "터미널 {count}개",
  "workbench.close.confirmGroup":
    "“{title}”의 터미널 {count}개에서 명령을 실행 중일 수 있습니다. 닫으면 연결을 끊고 세션 로그를 저장합니다. 계속할까요?",
  "workbench.close.confirmTerminal":
    "“{title}”에서 명령을 실행 중일 수 있습니다. 닫으면 연결을 끊고 세션 로그를 저장합니다. 계속할까요?",
  "workbench.layout.swapVertical": "위아래 바꾸기",
  "workbench.layout.swapHorizontal": "좌우 바꾸기",
  "workbench.actions.closeNamed": "{name} 닫기",
  "workbench.broadcast.disable": "브로드캐스트 중지(터미널 {broadcastCount}개)",
  "workbench.broadcast.commandExcluded":
    "명령 터미널은 브로드캐스트 입력을 수신하지 않습니다",
  "workbench.actions.duplicateTab": "탭 복제",
  "workbench.tabs.label": "애플리케이션 탭",
  "workbench.broadcast.enable": "터미널 {broadcastCount}개에 입력 브로드캐스트",
  "workbench.broadcast.unsupported":
    "이 터미널에서는 입력 브로드캐스트를 사용할 수 없습니다.",
  "workbench.actions.split": "세션 분할",
  "workbench.actions.newTabWithShortcut": "새 탭(Cmd/Ctrl+T)",
  "workbench.broadcast.minimumTerminals":
    "먼저 대화형 터미널을 두 개 이상 연결하세요",
  "workbench.layout.resizeVertical": "위아래 터미널 크기 조절",
  "workbench.layout.resizeHorizontal": "좌우 터미널 크기 조절",
} as const;

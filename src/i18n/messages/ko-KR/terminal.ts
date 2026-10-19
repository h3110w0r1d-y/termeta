// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "terminal.regions.area": "터미널 영역",
  "terminal.search.label": "터미널 기록 검색",
  "terminal.search.placeholder": "터미널 기록 검색…",
  "terminal.search.close": "검색 닫기",
  "terminal.actions.copySelection": "선택 영역 복사",
  "terminal.actions.paste": "붙여넣기",
  "terminal.actions.find": "터미널에서 찾기",
  "terminal.actions.scrollToBottom": "맨 아래로 스크롤",
  "terminal.actions.openLink": "링크 열기",
  "terminal.titles.local": "로컬 터미널",
  "terminal.snapshot.header": "\r\n[연결 종료 전 전체 화면 스냅샷]\r\n",
  "terminal.auth.title": "SSH 인증",
  "terminal.snapshot.truncated": "[전체 화면 스냅샷이 1 MiB에서 잘렸습니다]",
  "terminal.errors.unsentInput":
    "{error}; 전송하지 못한 입력은 자동으로 다시 전송되지 않습니다.",
  "terminal.actions.closeNamed": "터미널 {name} 닫기",
  "terminal.errors.fontLoad":
    "터미널 글꼴을 불러오지 못했습니다. 설정을 다시 적용하거나 앱을 다시 시작하세요.",
  "terminal.state.created": "연결 준비 완료",
  "terminal.paste.tooLarge": "한 번에 붙여넣을 수 있는 크기는 64 KiB입니다.",
  "terminal.actions.cancelConnection": "연결 취소",
  "terminal.state.startingLocal": "로컬 터미널 시작 중",
  "terminal.titles.command": "명령",
  "terminal.state.commandFinished":
    "명령이 종료되었습니다. 다시 실행하려면 컨텍스트 메뉴를 사용하세요.",
  "terminal.auth.answerTooLong": "응답은 16,384자를 초과할 수 없습니다",
  "terminal.paste.broadcastDescription":
    "터미널 {count}개에 {lines}줄 붙여넣기",
  "terminal.paste.currentDescription": "이 터미널에 {lines}줄 붙여넣기",
  "terminal.state.closed": "연결 닫힘",
  "terminal.state.disconnected": "연결 끊김",
  "terminal.state.connected": "연결됨",
  "terminal.state.startingSsh": "SSH 연결 시작 중",
  "terminal.state.startingTelnet": "Telnet 연결 시작 중(암호화 안 됨)",
  "terminal.state.opening": "터미널 여는 중",
  "terminal.paste.warning":
    "줄바꿈으로 인해 원격 명령이 즉시 실행될 수 있습니다. 계속하기 전에 모든 내용을 확인하세요.",
  "terminal.search.history": "터미널 기록 검색",
  "terminal.actions.disconnect": "연결 끊기",
  "terminal.errors.outputRead":
    "터미널 출력을 읽을 수 없습니다. 다시 연결하세요.",
  "terminal.errors.sessionClosing":
    "이전 연결을 닫는 중입니다. 잠시 후 다시 시도하세요.",
  "terminal.errors.inputUnavailable":
    "입력을 전송할 수 없습니다. 다시 연결하세요.",
  "terminal.state.closing": "연결 끊는 중",
  "terminal.state.unavailable": "연결 중이거나 연결이 종료됨",
  "terminal.paste.confirmMultiline": "여러 줄 붙여넣기 확인",
  "terminal.paste.confirmBroadcast": "터미널 {targetCount}개에 브로드캐스트",
  "terminal.paste.confirmCurrent": "이 터미널에 붙여넣기",
  "terminal.state.awaitingHostKey": "호스트 확인 대기 중",
  "terminal.state.awaitingAuth": "인증 대기 중",
  "terminal.titles.terminal": "터미널",
  "terminal.errors.outputPending":
    "터미널 출력을 처리하고 있습니다. 잠시 후 다시 시도하세요. 기록은 보존되었습니다.",
  "terminal.errors.outputFailure":
    "터미널 출력이 중단되었습니다. 다시 연결하세요.",
  "terminal.errors.tcpConnectFailed": "호스트와 포트에 연결할 수 없습니다.",
  "terminal.form.theme": "터미널 색 구성표",
  "terminal.accessibility.title": "터미널: {name}",
  "terminal.state.authenticating": "인증 중",
  "terminal.theme.hostDefault": "호스트 / 기본 색 구성표 사용",
  "terminal.paste.queueFull": "입력 처리 중입니다. 잠시 후 다시 붙여넣으세요.",
  "terminal.state.connecting": "연결 중",
  "terminal.state.failed": "연결 실패",
  "terminal.state.ended": "연결 종료",
  "terminal.auth.target": "연결 대상: {targetTitle}",
  "terminal.state.incompleteOutput":
    "연결이 종료되었습니다. 마지막 출력이 불완전할 수 있습니다.",
  "terminal.actions.reopenLocal":
    "로컬 터미널을 다시 여는 중입니다. 터미널 기록은 보존됩니다.",
  "terminal.actions.rerun": "다시 실행",
  "terminal.actions.reconnect": "다시 연결",
  "terminal.actions.reconnectWithHistory":
    "다시 연결하는 중입니다. 터미널 기록은 보존됩니다.",
  "terminal.exit.signal": " (시그널 {exitSignal})",
  "terminal.exit.code": " (종료 코드 {exitCode})",
} as const;

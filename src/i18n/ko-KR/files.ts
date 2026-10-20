// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "files.sidebar.toggle": "SFTP 표시 또는 숨기기",
  "files.errors.sshDisconnected":
    "SSH 연결이 끊어졌습니다. 다시 연결한 후 새로 고침하세요.",
  "files.count": "항목 {count}개",
  "files.countPartial": "항목 {count}개(더 있음)",
  "files.transfer.summary":
    "{name} · 파일 {completed}/{total}개 · {transferred}{size}",
  "files.actions.upload": "업로드",
  "files.actions.parent": "상위 폴더",
  "files.actions.download": "다운로드",
  "files.actions.home": "홈 폴더",
  "files.transfer.active": "전송 중",
  "files.transfer.progress": "전송 중 {completed}/{total}",
  "files.transfer.complete": "전송 완료",
  "files.transfer.queue": "전송 대기열",
  "files.types.other": "기타 파일",
  "files.copy.preparing": "복사 준비 중…",
  "files.errors.staleList": "파일 목록이 오래되었습니다. 새로 고침하세요.",
  "files.actions.go": "이동",
  "files.actions.loadMorePage": "더 불러오기(100개)",
  "files.browser.title": "이중 창 SFTP 탐색기",
  "files.actions.cancelTransfer": "전송 취소",
  "files.actions.cancelCopy": "복사 취소",
  "files.copy.unsupportedType":
    "일반 파일과 폴더만 복사할 수 있습니다. 심볼릭 링크는 아직 지원하지 않습니다.",
  "files.copy.dragHint": "항목 {count}개 복사 · 다른 창으로 드래그",
  "files.copy.confirm": "항목 {count}개를 다른 폴더에 복사",
  "files.actions.copyToOtherPane": "다른 창으로 복사",
  "files.fields.size": "크기",
  "files.copy.complete": "복사 완료",
  "files.upload.confirmDrop":
    "놓은 파일이나 폴더를 {target}에 업로드할까요? 같은 이름의 폴더는 병합됩니다. 기존 파일을 덮어쓸 때마다 확인합니다.",
  "files.transfer.bytes": "전송됨 {transferred} / {total}",
  "files.selection.loaded": "{loaded}개 불러옴 · {selected}개 선택됨",
  "files.transfer.removed": "대기열에서 제거됨",
  "files.selection.count": "항목 {count}개 선택됨",
  "files.selection.named": "선택됨: {name}",
  "files.transfer.queueFull":
    "대기열이 최대 32개에 도달했습니다. 전송이 완료될 때까지 기다리세요.",
  "files.types.file": "파일",
  "files.types.directory": "폴더",
  "files.types.directoryLink": "폴더 심볼릭 링크",
  "files.types.fileLink": "파일 심볼릭 링크",
  "files.errors.tooManyEntries":
    "파일이 너무 많아 표시할 수 없습니다. 하위 폴더를 열어 계속 탐색하세요.",
  "files.confirmDisconnect":
    "SSH 연결을 끊으면 현재 SFTP 전송이 취소됩니다. 계속할까요?",
  "files.actions.showHidden": "숨김 파일 표시",
  "files.fields.permissions": "권한",
  "files.upload.dropHint": "놓아서 {path}에 업로드",
  "files.transfer.inProgress": "전송 중",
  "files.confirmCloseTerminal":
    "SFTP 전송이 진행 중입니다. 닫으면 취소됩니다. 계속할까요?",
  "files.sidebar.label": "이 터미널의 SFTP 탐색기",
  "files.errors.dropBusy":
    "현재 작업이 끝날 때까지 기다린 후 파일을 다시 드래그하세요.",
  "files.path.placeholder": "원격 경로 입력",
  "files.sidebar.resize": "SFTP 사이드바 크기 조절",
  "files.types.unknownLink": "심볼릭 링크(대상 유형 알 수 없음)",
  "files.path.label": "원격 경로",
  "files.transfer.connectionChanged":
    "연결이 변경되었습니다. 전송을 시작하지 않았습니다.",
  "files.hostPlaceholder": "파일을 탐색할 호스트 선택",
  "files.transfer.preparing": "파일 선택 또는 전송 준비 중…",
  "files.sidebar.hide": "SFTP 사이드바 숨기기",
  "files.transfer.hidePanel": "패널을 숨기고 전송 계속",
} as const;

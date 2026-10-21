// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "files.hosts.search": "SSH 호스트 검색…",
  "files.hosts.empty":
    "저장된 SSH 호스트가 없습니다. 먼저 보관함에 추가하세요.",
  "files.drag.pastePaths": "놓아서 경로 {count}개 붙여넣기",
  "files.drag.hint":
    "{count}개 항목 · 터미널에 놓아 경로 붙여넣기, 창 밖으로 끌어 다운로드",
  "files.drag.invalidPaths":
    "선택한 경로가 너무 길거나 지원하지 않는 문자가 포함되어 있습니다. 항목을 줄여 다시 시도하세요.",
  "files.follow.label": "터미널 작업 디렉터리 따라가기",
  "files.follow.description":
    "터미널에서 디렉터리 변경을 알리면 SFTP를 동기화합니다. 파일 작업이 끝나면 최신 디렉터리로 이동합니다.",
  "files.follow.waiting":
    "터미널 디렉터리를 기다리는 중입니다. 셸에서 디렉터리를 알리거나 제목에 전체 경로를 표시해야 합니다.",
  "files.navigation.collapsed": "생략된 상위 폴더 표시",
  "files.navigation.edit": "폴더 경로 편집",
  "files.activity.title": "전송 기록",
  "files.activity.open": "전송 목록 보기",
  "files.activity.active": "{count}개 전송 중",
  "files.activity.clearFinished": "종료된 전송 지우기",
  "files.activity.empty": "아직 전송 내역이 없습니다",
  "files.activity.idle": "모든 전송이 종료되었습니다",
  "files.activity.failed": "전송 실패",
  "files.activity.cancelling": "취소 중…",
  "files.activity.progress": "전송 진행률",
  "files.activity.unknownProgress":
    "준비 중이며 전체 진행률은 아직 확인할 수 없습니다",
  "files.table.label": "원격 파일 목록",
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
  "files.manage.delete.action": "삭제",
  "files.manage.delete.title": "선택한 항목 삭제",
  "files.manage.delete.recursiveTitle": "폴더와 내용 삭제 확인",
  "files.manage.delete.warning":
    "선택한 항목이 원격 호스트에서 영구 삭제되며 되돌릴 수 없습니다. 링크는 링크 자체만 삭제합니다.",
  "files.manage.delete.recursiveWarning":
    "선택한 폴더와 숨김 파일을 포함한 모든 내용이 영구 삭제됩니다. 중단되거나 실패해도 이미 삭제된 항목은 복구할 수 없습니다. 하위 항목까지 삭제할까요?",
  "files.manage.delete.continue": "폴더 삭제 확인으로 계속",
  "files.manage.delete.permanent": "영구 삭제",
  "files.manage.selection": "{count}개 항목 선택됨",
  "files.manage.partialFailure":
    "일부 변경이 이미 완료되었을 수 있습니다. 새로 고친 목록을 확인한 후 다시 시도하세요. 완료된 변경은 되돌릴 수 없습니다.",
  "files.manage.limit": "한 번에 최대 100개 항목을 선택할 수 있습니다.",
  "files.manage.permissions.title": "권한 편집",
  "files.manage.permissions.applies":
    "선택한 모든 항목에 적용됩니다. 폴더의 내용은 재귀적으로 변경하지 않습니다. 현재 권한이 서로 다르거나 알 수 없는 경우 값을 명시적으로 설정하세요.",
  "files.manage.permissions.owner": "소유자",
  "files.manage.permissions.group": "그룹",
  "files.manage.permissions.others": "다른 사용자",
  "files.manage.permissions.read": "읽기",
  "files.manage.permissions.write": "쓰기",
  "files.manage.permissions.execute": "실행 / 진입",
  "files.manage.permissions.code": "8진수 권한",
  "files.manage.permissions.invalid":
    "3자리 또는 4자리 8진수 권한(000~7777)을 입력하세요.",
  "files.manage.permissions.special": "특수 권한 (고급)",
  "files.manage.permissions.setuid": "Setuid: 소유자로 실행",
  "files.manage.permissions.setgid": "Setgid: 그룹 상속 / 그룹으로 실행",
  "files.manage.permissions.sticky":
    "Sticky: 디렉터리 내 항목 삭제를 소유자로 제한",
  "files.manage.delete.confirmContents":
    "선택한 폴더와 모든 내용이 영구 삭제됨을 이해했습니다.",
} as const;

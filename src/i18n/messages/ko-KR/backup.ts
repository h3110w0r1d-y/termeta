// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "backup.title": "백업 및 동기화",
  "backup.form.provider": "백업 서비스",
  "backup.form.deviceName": "기기 이름",
  "backup.form.devicePlaceholder": "예: 내 MacBook",
  "backup.form.webdavUrl": "WebDAV 폴더 URL(HTTPS)",
  "backup.form.password": "백업 비밀번호(12자 이상)",
  "backup.form.confirmPassword": "백업 비밀번호 확인",
  "backup.form.operation": "백업 또는 동기화",
  "backup.empty": "이 폴더 또는 계정에 백업이 없습니다.",
  "backup.import.title": "클라우드 가져오기 확인",
  "backup.description":
    "호스트, 그룹, 자격 증명, 포워딩 규칙 및 신뢰하는 호스트 키를 암호화하여 백업합니다. 앱 설정은 이 기기에 유지됩니다. 백업 비밀번호를 안전하게 보관하세요.",
  "backup.passwordHint":
    "백업 비밀번호는 이 기기에 저장됩니다. 변경 사항은 새 백업에만 적용되며 이전 백업에는 원래 비밀번호가 필요합니다.",
  "backup.actions.test": "연결 테스트",
  "backup.actions.saveConfig": "클라우드 설정 저장",
  "backup.actions.uploadVersion": "백업 · 새 버전 업로드",
  "backup.actions.downloadVersion": "동기화 · 클라우드에서 다운로드",
  "backup.actions.preview": "다운로드 후 미리 보기",
  "backup.actions.loadMore": "백업 더 불러오기",
  "backup.form.githubToken": "GitHub 토큰(Gist 권한 필요)",
  "backup.form.webdavPassword": "WebDAV 비밀번호 / 앱 비밀번호",
  "backup.form.savedSecretPlaceholder":
    "저장됨. 비워 두면 유지하고 입력하면 교체합니다",
  "backup.form.credentialsHint": "자격 증명은 이 기기에서 암호화됩니다",
  "backup.form.passwordHint": "백업 비밀번호는 이 기기에서 암호화됩니다",
  "backup.version.undecryptable": "백업을 복호화할 수 없습니다",
  "backup.version.decryptError": "비밀번호가 틀렸거나 백업이 손상되었습니다.",
  "backup.upload.description": "업로드할 때마다 새 백업 버전을 만듭니다.",
  "backup.download.description":
    "버전을 선택하고 미리 본 후 가져오기를 확인하세요.",
  "backup.upload.progress": "백업 암호화 후 업로드",
  "backup.download.progress": "클라우드 백업 불러오기",
  "backup.import.setupDescription":
    "로컬 보관함을 만들고 선택한 백업을 가져옵니다. 보관함 비밀번호와 백업 비밀번호는 별개입니다. 둘 다 안전하게 보관하세요.",
  "backup.import.mergeDescription":
    "기존 로컬 데이터와 설정을 유지하며 데이터를 추가합니다. 다시 가져오면 중복이 생깁니다. 충돌하는 호스트 키는 경고와 함께 건너뜁니다.",
  "backup.actions.setupAndImport": "보관함 만들기 및 가져오기",
  "backup.actions.confirmImport": "새 데이터로 가져오기",
  "backup.import.counts":
    "호스트 {hosts} · 그룹 {groups} · 자격 증명 {credentials} · 포워딩 {forwards} · 신뢰하는 키 {knownHosts} · 스니펫 {snippets}",
  "backup.import.success":
    "동기화 완료: {counts}. 기존 로컬 데이터와 설정은 유지되었습니다.",
  "backup.defaultDeviceName": "내 기기",
  "backup.validation.deviceName": "기기 이름을 입력하세요(최대 256바이트)",
  "backup.validation.urlFormat":
    "자격 증명, 쿼리 매개변수 및 프래그먼트가 없는 HTTPS 폴더 URL을 입력하세요",
  "backup.validation.webdavUrl": "올바른 WebDAV HTTPS 폴더 URL을 입력하세요",
  "backup.validation.username":
    "콜론이 없는 사용자 이름을 입력하세요(최대 1,024바이트)",
  "backup.validation.credentials": "클라우드 자격 증명을 입력하세요",
  "backup.validation.passwordRequired": "백업 비밀번호를 입력하세요",
  "backup.validation.passwordMaxLength":
    "비밀번호는 1,024바이트를 초과할 수 없습니다",
  "backup.validation.passwordMinLength":
    "백업 비밀번호는 12자 이상이어야 합니다",
  "backup.validation.passwordMismatch": "백업 비밀번호가 일치해야 합니다",
  "backup.status.configSaved":
    "클라우드 설정을 암호화하여 이 기기에 저장했습니다.",
  "backup.status.connected": "연결에 성공했습니다.",
  "backup.status.uploaded":
    "새 백업을 업로드했습니다. 기존 클라우드 백업은 유지되었습니다.",
  "backup.errors.setupUnavailable":
    "초기 설정을 사용할 수 없습니다. 앱을 다시 열고 시도하세요.",
  "backup.validation.localPassword":
    "가져오기 전에 위에서 로컬 보관함 비밀번호를 입력하고 확인하세요.",
} as const;

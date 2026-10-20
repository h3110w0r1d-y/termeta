export default {
  "account.oauth.confirmationCode": "브라우저 일회용 확인 코드",
  "account.oauth.codeDescription":
    "브라우저에서 Termeta를 열 수 없으면 인증 완료 페이지에서 일회용 확인 코드를 복사해 아래에 입력하세요.",
  "account.oauth.manualFallback": "자동으로 돌아오지 않나요? 확인 코드 입력",
  "account.oauth.returnHint":
    "브라우저에서 Termeta를 열지 물으면 허용하세요. 자동으로 돌아오지 않으면 페이지의 “Termeta로 돌아가기”를 선택하세요. 검증이 완료될 때까지 이 창을 열어 두세요.",
  "account.oauth.codeSecurity":
    "본인이 이 기기에서 시작한 인증 코드만 입력하세요. 코드를 공유하거나 다른 사람을 대신해 입력하지 마세요.",
  "account.oauth.submitCode": "확인 코드 제출",
  "account.oauth.codeSubmitted":
    "확인 코드를 제출했습니다. 검증이 완료될 때까지 이 창을 열어 두세요.",
  "account.oauth.validationCode":
    "브라우저에 표시된 영문자, 숫자, 밑줄 또는 하이픈으로 된 32자 확인 코드를 입력하세요.",
  "account.title": "계정",
  "account.description":
    "공식 계정 모드는 보관함 전체를 termeta.io에 자동 동기화합니다. 로컬 모드에는 계정이 필요하지 않습니다.",
  "account.actions.logout": "로그아웃",
  "account.actions.loginProvider": "{provider}로 로그인",
  "account.actions.retryProviders": "로그인 방법 다시 불러오기",
  "account.actions.cancelLogin": "대기 취소",
  "account.actions.openSettings": "계정 및 설정",
  "account.logout.confirm":
    "로그아웃하고 로컬 모드로 전환할까요? 로컬 데이터와 터미널 세션은 유지되며 공식 자동 동기화가 중지됩니다.",
  "account.oauth.waiting":
    "시스템 브라우저에서 {provider} 인증을 완료하세요. 이후 Termeta로 자동으로 돌아와 검증을 마칩니다.",
  "account.oauth.deadline": "대기 기한: {date}",
  "account.errors.expired": "인증이 만료되었습니다. 다시 시작하세요.",
  "account.errors.denied":
    "인증이 완료되지 않았습니다. 다시 시도하거나 다른 로그인 방법을 사용하세요.",
  "account.mode.title": "Termeta 사용 모드",
  "account.mode.description":
    "공식 계정으로 로그인하여 보관함을 termeta.io에 자동 동기화하거나 로컬 오프라인 모드에서 직접 데이터를 관리합니다.",
  "account.mode.official": "공식 계정 모드",
  "account.mode.local": "로컬 / 오프라인 모드",
  "account.mode.logoutLocal": "로그아웃하고 로컬 모드 사용",
  "account.browser.description":
    "계정 가입, 로그인 및 로그인 방법 관리는 브라우저의 termeta.io에서 진행합니다.",
  "account.sync.unlockTitle": "클라우드 보관함 잠금 해제",
  "account.sync.unlockDescription":
    "클라우드에 데이터가 있습니다. 보관함 비밀번호를 입력하면 검증 후 로컬 데이터와 병합하여 계속합니다. 비밀번호가 틀리면 진입할 수 없습니다. 로그아웃하면 로컬 데이터를 유지하고 로컬 모드를 사용할 수 있습니다.",
  "account.sync.setupDescription":
    "로컬 및 공식 클라우드 데이터를 함께 보호할 보관함 비밀번호를 설정하세요. 비밀번호를 기억하세요. 계정 로그인으로 잊어버린 보관함 비밀번호를 복구할 수 없습니다.",
  "account.sync.existingDescription":
    "현재 로컬 보관함 비밀번호를 입력하여 공식 자동 동기화를 활성화하세요. 공식 클라우드와 로컬 보관함은 동일한 비밀번호를 사용합니다.",
  "account.sync.description":
    "공식 클라우드로 전체 자동 동기화가 활성화되어 있으며 로컬 보관함과 같은 마스터 키를 사용합니다. 로그아웃하면 로컬 데이터를 유지하고 공식 동기화를 중지합니다.",
  "account.sync.mainWindowRequired":
    "Termeta 기본 창으로 돌아가 보관함 비밀번호를 입력하여 클라우드 데이터를 복호화하고 병합하세요. 검증이 완료될 때까지 작업대에 진입할 수 없습니다.",
  "account.sync.lastSynced": "마지막 동기화: {date}",
  "account.sync.now": "지금 동기화",
  "account.sync.continue": "검증 후 계속",
  "account.sync.localUnlockTitle": "먼저 로컬 보관함 잠금 해제",
  "account.sync.localUnlockDescription":
    "로컬 데이터를 유지하고 병합하려면 현재 로컬 보관함 비밀번호를 입력하세요. 다른 기기에서 클라우드 비밀번호를 변경했다면 이전 로컬 비밀번호와 다를 수 있습니다. 로그아웃할 필요 없이 로컬 잠금 해제 후 클라우드 비밀번호를 검증합니다.",
} as const;

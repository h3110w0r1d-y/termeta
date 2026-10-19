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

  "account.actions.clearLogin": "로그아웃 / 로컬 로그인 지우기",
  "account.title": "계정",
  "account.description":
    "이메일과 비밀번호, GitHub 또는 Google로 공식 Termeta 계정에 로그인하세요. 로컬 터미널은 계정 없이 사용할 수 있습니다.",
  "account.encryptionHint":
    "로그인은 보관함 잠금을 해제하지 않으며 보관함 비밀번호나 백업 암호화 비밀번호를 대체하지 않습니다.",
  "account.admin": "관리자",
  "account.createdAt": "가입 일시: {date}",
  "account.expiresAt": "세션 만료: {date}",
  "account.signedOut": "로그인하지 않았습니다.",
  "account.actions.refresh": "계정 상태 새로 고침",
  "account.actions.logout": "로그아웃",
  "account.actions.login": "로그인",
  "account.actions.register": "가입",
  "account.actions.retry": "계정 다시 읽기",
  "account.actions.unlink": "연결 해제",
  "account.actions.linkProvider": "{provider} 연결",
  "account.actions.loginProvider": "{provider}로 로그인",
  "account.actions.retryProviders": "로그인 방법 다시 불러오기",
  "account.actions.cancelLogin": "대기 취소",
  "account.actions.openSettings": "계정 및 설정",
  "account.identities.title": "로그인 방법",
  "account.identities.description":
    "연결하기 전에 브라우저에서 외부 계정을 확인하세요. 이메일이 같아도 계정은 자동으로 병합되지 않습니다.",
  "account.identities.password": "이메일과 비밀번호",
  "account.identities.lastMethod": "로그인 방법을 하나 이상 유지해야 합니다.",
  "account.unlink.confirm":
    "{provider} 연결을 해제할까요? 해제하면 해당 계정으로 Termeta에 로그인할 수 없습니다.",
  "account.logout.confirm":
    "공식 계정에서 로그아웃할까요? 로컬 보관함과 터미널 세션은 계속 사용할 수 있습니다.",
  "account.oauth.completed": "인증이 완료되어 계정 상태가 업데이트되었습니다.",
  "account.oauth.cancelled": "브라우저 인증을 취소했습니다.",
  "account.oauth.waiting":
    "시스템 브라우저에서 {provider} 인증을 완료하세요. 이후 Termeta로 자동으로 돌아와 검증을 마칩니다.",
  "account.oauth.deadline": "대기 기한: {date}",
  "account.errors.expired": "인증이 만료되었습니다. 다시 시작하세요.",
  "account.errors.denied":
    "인증이 완료되지 않았습니다. 다시 시도하거나 다른 로그인 방법을 사용하세요.",
  "account.form.email": "이메일",
  "account.form.password": "비밀번호",
  "account.form.confirmation": "비밀번호 확인",
  "account.validation.email": "유효한 이메일 주소를 입력하세요.",
  "account.validation.password": "비밀번호는 12~256자여야 합니다.",
  "account.validation.confirmation": "비밀번호가 일치하지 않습니다.",
} as const;

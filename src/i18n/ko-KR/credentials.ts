// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "credentials.validation.passwordRequired": "비밀번호를 입력하세요",
  "credentials.confirmDelete":
    "보관함에서 자격 증명 “{label}”을(를) 삭제할까요? 이 작업은 취소할 수 없습니다.",
  "credentials.description":
    "여러 호스트에서 재사용할 비밀번호와 개인 키를 저장하세요.",
  "credentials.actions.new": "새 자격 증명",
  "credentials.search.label": "자격 증명 검색",
  "credentials.search.placeholder": "자격 증명 이름 검색…",
  "credentials.status.loading": "자격 증명 불러오는 중…",
  "credentials.search.noResults":
    "일치하는 자격 증명이 없습니다. 다른 검색어를 입력하세요.",
  "credentials.empty":
    "저장된 자격 증명이 없습니다. 호스트 연결에 사용할 자격 증명을 만드세요.",
  "credentials.actions.create": "자격 증명 만들기",
  "credentials.types.password": "비밀번호",
  "credentials.types.privateKey": "개인 키",
  "credentials.status.ready": "준비됨",
  "credentials.status.awaitingRecovery": "복구 대기 중",
  "credentials.status.pendingDeletion": "삭제 대기 중",
  "credentials.delete.referenceHint":
    "먼저 호스트 설정에서 이 자격 증명 사용을 해제하세요",
  "credentials.actions.delete": "자격 증명 삭제",
  "credentials.actions.retryDelete": "삭제 다시 시도",
  "credentials.form.description":
    "호스트 인증에 사용하도록 보관함에 안전하게 저장됩니다.",
  "credentials.form.namePlaceholder": "예: 프로덕션 SSH 비밀번호",
  "credentials.form.sshPassword": "SSH 비밀번호",
  "credentials.form.sshPrivateKey": "SSH 개인 키",
  "credentials.form.privateKeyPlaceholder":
    "전체 SSH 개인 키를 붙여넣으세요(OpenSSH, PEM 또는 PuTTY PPK)",
  "credentials.form.passphrase": "키 암호(선택 사항)",
  "credentials.form.passphraseHint":
    "비워 두면 필요한 경우 연결할 때 입력을 요청합니다",
  "credentials.actions.save": "자격 증명 저장",
  "credentials.delete.description":
    "삭제하기 전에 호스트 설정에서 자격 증명 사용을 해제하세요.",
  "credentials.form.keyHint":
    "개인 키는 암호화되어 보관함에 저장됩니다. 최대 크기: 1 MiB.",
  "credentials.referenceCount": "호스트 {count}개에서 사용 중",
  "credentials.actions.showPublicKey": "공개 키 보기",
  "credentials.actions.copyPublicKey": "공개 키 복사",
  "credentials.publicKey.title": "SSH 공개 키",
  "credentials.publicKey.loading": "공개 키 불러오는 중…",
  "credentials.publicKey.hint":
    "서버의 authorized_keys 파일에 추가할 수 있습니다.",
  "credentials.publicKey.copied": "공개 키를 복사했습니다",
  "credentials.publicKey.passphraseHint":
    "개인 키가 암호화되어 있으면 암호를 입력하고 다시 시도하세요. 입력한 암호는 공개 키 확인에만 사용되며 저장되지 않습니다.",
} as const;

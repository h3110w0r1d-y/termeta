// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "vault.validation.passwordRequired": "보관함 비밀번호를 입력하세요",
  "vault.validation.passwordMismatch": "비밀번호가 일치해야 합니다.",
  "vault.setup.title": "보관함 비밀번호 만들기",
  "vault.unlock.title": "보관함 잠금 해제",
  "vault.access.checkingTitle": "보관함 복원 중",
  "vault.access.checkingDescription":
    "이 기기의 보관함과 계정 상태를 확인하고 있습니다. 잠시 기다려 주세요…",
  "vault.access.failedTitle": "보관함을 열 수 없습니다",
  "vault.access.failedDescription":
    "오류 내용을 확인하고 다시 시도하세요. 기존 데이터는 보존됩니다.",
  "vault.setup.method": "보관함 만들기",
  "vault.setup.empty": "빈 보관함 만들기",
  "vault.setup.cloud": "클라우드 백업 복원",
  "vault.setup.passwordPlaceholder": "비밀번호(12자 이상)",
  "vault.form.password": "보관함 비밀번호",
  "vault.form.confirmPassword": "비밀번호 확인",
  "vault.actions.createAndUnlock": "만들고 잠금 해제",
  "vault.actions.unlock": "잠금 해제",
  "vault.validation.passwordLength": "12자 이상, 1024바이트 이하로 입력하세요.",
  "vault.setup.warning":
    "호스트와 자격 증명은 이 기기에서 암호화됩니다. 비밀번호를 안전하게 보관하세요. 자동 잠금 해제를 사용할 수 없고 비밀번호를 잊은 경우 데이터를 복구할 수 없습니다.",
  "vault.unlock.automaticProgress":
    "자동 잠금 해제를 시도하고 있습니다. 사용할 수 없는 경우 보관함 비밀번호를 입력하세요.",
  "vault.password.title": "보관함 비밀번호 변경",
  "vault.password.warning":
    "새 비밀번호를 기억하세요. 공식 클라우드 동기화도 새 비밀번호를 사용합니다. 기존 오프라인 사본에는 이전 비밀번호가 필요하며 타사 백업은 별도의 비밀번호를 유지합니다.",
  "vault.password.old": "현재 비밀번호",
  "vault.password.new": "새 비밀번호",
  "vault.password.confirm": "새 비밀번호 확인",
  "vault.password.description":
    "비밀번호를 변경하면 이 기기에 저장된 호스트, 자격 증명, 설정 및 기록을 다시 암호화합니다.",
  "vault.password.success": "보관함 비밀번호를 변경했습니다.",
  "vault.password.progress":
    "데이터를 다시 암호화하고 있습니다. 잠시 기다려 주세요…",
  "vault.password.submit": "비밀번호 변경",
  "vault.password.validation.old":
    "현재 비밀번호를 입력하세요(최대 1,024바이트)",
  "vault.password.validation.length":
    "12자 이상, 1,024바이트 이하로 입력하세요",
  "vault.password.validation.unchanged":
    "현재 비밀번호와 다른 비밀번호를 선택하세요",
  "vault.password.validation.mismatch": "새 비밀번호가 일치해야 합니다",
} as const;

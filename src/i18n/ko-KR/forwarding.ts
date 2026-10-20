// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "forwarding.validation.nameLength": "이름은 128자 이하여야 합니다",
  "forwarding.form.bindAddress": "수신 대기 IP 주소",
  "forwarding.form.bindPort": "수신 대기 포트",
  "forwarding.form.targetAddress": "대상 주소",
  "forwarding.form.targetPort": "대상 포트",
  "forwarding.actions.edit": "포트 포워딩 편집",
  "forwarding.actions.new": "새 포트 포워딩",
  "forwarding.form.namePlaceholder":
    "예: 데이터베이스, 개발 서비스 또는 프록시",
  "forwarding.form.hostHint": "먼저 호스트 페이지에서 SSH 호스트를 만드세요.",
  "forwarding.form.mode": "포워딩 모드",
  "forwarding.modes.local": "로컬 포워딩",
  "forwarding.modes.remote": "원격 포워딩",
  "forwarding.modes.dynamic": "동적 SOCKS5",
  "forwarding.form.remoteBindAddress": "원격 수신 대기 IP",
  "forwarding.form.localBindAddress": "로컬 수신 대기 IP",
  "forwarding.form.loopbackPlaceholder": "127.0.0.1 또는 ::1",
  "forwarding.form.localTargetAddress": "로컬 대상 주소",
  "forwarding.form.remoteTargetAddress": "원격 대상 주소",
  "forwarding.form.targetPlaceholder":
    "호스트 이름 또는 IP. IPv6에는 대괄호를 생략하세요",
  "forwarding.confirmDelete":
    "포트 포워딩 “{name}”을(를) 삭제할까요? SSH 호스트는 유지됩니다.",
  "forwarding.actions.create": "새 포워딩",
  "forwarding.search.placeholder": "이름, SSH 호스트 또는 포워딩 주소 검색",
  "forwarding.search.label": "포트 포워딩 검색",
  "forwarding.search.noResults": "일치하는 포워딩 규칙이 없습니다",
  "forwarding.empty": "포워딩 규칙이 없습니다",
  "forwarding.status.stopped": "중지됨",
  "forwarding.errors.hostDeleted": "SSH 호스트가 삭제되었습니다",
  "forwarding.endpoints.local": "로컬",
  "forwarding.endpoints.dynamicTarget": "SOCKS5 → 애플리케이션에서 대상 선택",
  "forwarding.endpoints.localSide": "로컬",
  "forwarding.endpoints.remoteSide": "원격",
  "forwarding.endpoints.target": "→ {value1} {value2}",
  "forwarding.descriptions.local":
    "로컬 포트를 통해 원격 대상에 접근합니다. SSH 서버가 대상 주소를 해석합니다.",
  "forwarding.descriptions.remote":
    "서버 포트를 통해 로컬 측 대상에 접근합니다. 이 컴퓨터가 대상 주소를 해석합니다.",
  "forwarding.descriptions.dynamic":
    "이 컴퓨터에서만 사용할 수 있는 SOCKS5 프록시를 만듭니다. SSH 서버가 호스트 이름을 해석합니다. 인증 없이 TCP만 지원합니다.",
  "forwarding.warning.publicListener":
    "0.0.0.0 또는 ::에서 수신 대기하면 다른 기기가 연결할 수 있습니다. 원격 접근은 서버 설정에도 영향을 받습니다.",
} as const;

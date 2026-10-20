// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "knownHosts.confirmRevoke":
    "신뢰를 취소할까요? 다음에 연결할 때 지문을 다시 확인해야 합니다.",
  "knownHosts.empty": "신뢰하는 호스트가 아직 없습니다.",
  "knownHosts.actions.revoke": "신뢰 취소",
  "knownHosts.description": "신뢰하는 SSH 호스트 관리",
  "knownHosts.fingerprint": "SHA256 지문",
  "knownHosts.actions.trustOnce": "이번만 신뢰",
  "knownHosts.actions.trustAndSave": "신뢰하고 저장",
  "knownHosts.actions.reject": "거부",
  "knownHosts.changedWarning":
    "서버 지문이 변경되면 연결이 차단됩니다. 연결하기 전에 새 지문을 확인하세요.",
  "knownHosts.verifyTitle": "SSH 호스트 신원 확인",
  "knownHosts.algorithm": "알고리즘",
  "knownHosts.verifyDescription":
    "신뢰할 수 있는 경로로 서버 지문을 확인하세요. 확인이 완료된 후에만 인증 정보를 전송합니다.",
} as const;

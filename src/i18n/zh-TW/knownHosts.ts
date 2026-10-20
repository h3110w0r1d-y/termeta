// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "knownHosts.confirmRevoke": "撤銷信任？下次連線需要重新核驗指紋。",
  "knownHosts.empty": "尚未儲存主機信任。",
  "knownHosts.actions.revoke": "撤銷信任",
  "knownHosts.description": "管理已信任的 SSH 主機",
  "knownHosts.fingerprint": "SHA256 指紋",
  "knownHosts.actions.trustOnce": "僅本次信任",
  "knownHosts.actions.trustAndSave": "信任並儲存",
  "knownHosts.actions.reject": "拒絕",
  "knownHosts.changedWarning": "伺服器指紋變化時會阻止連線，請重新核實。",
  "knownHosts.verifyTitle": "核驗 SSH 主機身份",
  "knownHosts.algorithm": "演算法",
  "knownHosts.verifyDescription":
    "請透過可信渠道核對伺服器指紋。只有核驗後才會提交認證資訊。",
} as const;

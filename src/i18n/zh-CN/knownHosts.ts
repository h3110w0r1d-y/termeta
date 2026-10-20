// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "knownHosts.confirmRevoke": "撤销信任？下次连接需要重新核验指纹。",
  "knownHosts.empty": "尚未保存主机信任。",
  "knownHosts.actions.revoke": "撤销信任",
  "knownHosts.description": "管理已信任的 SSH 主机",
  "knownHosts.fingerprint": "SHA256 指纹",
  "knownHosts.actions.trustOnce": "仅本次信任",
  "knownHosts.actions.trustAndSave": "信任并保存",
  "knownHosts.actions.reject": "拒绝",
  "knownHosts.changedWarning": "服务器指纹变化时会阻止连接，请重新核实。",
  "knownHosts.verifyTitle": "核验 SSH 主机身份",
  "knownHosts.algorithm": "算法",
  "knownHosts.verifyDescription":
    "请通过可信渠道核对服务器指纹。只有核验后才会提交认证信息。",
} as const;

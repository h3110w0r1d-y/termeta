// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "knownHosts.confirmRevoke":
    "Revoke trust? You will need to verify the fingerprint again next time you connect.",
  "knownHosts.empty": "No trusted hosts yet.",
  "knownHosts.actions.revoke": "Revoke trust",
  "knownHosts.description": "Manage trusted SSH hosts",
  "knownHosts.fingerprint": "SHA256 fingerprint",
  "knownHosts.actions.trustOnce": "Trust once",
  "knownHosts.actions.trustAndSave": "Trust and save",
  "knownHosts.actions.reject": "Reject",
  "knownHosts.changedWarning":
    "Connections are blocked if the server fingerprint changes. Verify the new fingerprint before connecting.",
  "knownHosts.verifyTitle": "Verify SSH host identity",
  "knownHosts.algorithm": "Algorithm",
  "knownHosts.verifyDescription":
    "Verify the server fingerprint through a trusted channel. Authentication details will only be sent after verification.",
} as const;

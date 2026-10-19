// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "knownHosts.confirmRevoke":
    "信頼を取り消しますか？次回の接続時にフィンガープリントの再確認が必要になります。",
  "knownHosts.empty": "信頼済みのホストはありません。",
  "knownHosts.actions.revoke": "信頼を取り消す",
  "knownHosts.description": "信頼済みSSHホストを管理",
  "knownHosts.fingerprint": "SHA256フィンガープリント",
  "knownHosts.actions.trustOnce": "今回のみ信頼",
  "knownHosts.actions.trustAndSave": "信頼して保存",
  "knownHosts.actions.reject": "拒否",
  "knownHosts.changedWarning":
    "サーバーのフィンガープリントが変わると接続はブロックされます。接続前に新しいフィンガープリントを確認してください。",
  "knownHosts.verifyTitle": "SSHホストの識別情報を確認",
  "knownHosts.algorithm": "アルゴリズム",
  "knownHosts.verifyDescription":
    "信頼できる経路でサーバーのフィンガープリントを確認してください。認証情報は確認後にのみ送信されます。",
} as const;

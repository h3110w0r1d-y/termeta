// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "vault.validation.passwordRequired": "请输入保险库密码",
  "vault.validation.passwordMismatch": "两次输入的密码须一致",
  "vault.setup.title": "设置数据保险库密码",
  "vault.unlock.title": "解锁数据保险库",
  "vault.access.checkingTitle": "正在恢复保险库",
  "vault.access.checkingDescription": "正在检查本机保险库和账号状态，请稍候…",
  "vault.access.failedTitle": "暂时无法进入保险库",
  "vault.access.failedDescription": "请查看错误信息并重试。现有数据会保留。",
  "vault.setup.method": "创建保险库方式",
  "vault.setup.empty": "创建空保险库",
  "vault.setup.cloud": "从云端备份初始化",
  "vault.setup.passwordPlaceholder": "设置密码（至少12个字符）",
  "vault.form.password": "保险库密码",
  "vault.form.confirmPassword": "确认密码",
  "vault.actions.createAndUnlock": "创建并解锁",
  "vault.actions.unlock": "解锁",
  "vault.validation.passwordLength": "密码至少12个字符，且长度不超过1024字节",
  "vault.setup.warning":
    "主机和凭据会加密保存在本机。请牢记密码：如果自动解锁失效且忘记密码，数据将无法恢复。",
  "vault.unlock.automaticProgress":
    "正在尝试自动解锁。如未成功，请输入保险库密码。",
  "vault.password.title": "修改保险库密码",
  "vault.password.warning":
    "请牢记新密码。官方云端同步也将使用新密码；已有离线副本仍需旧密码，第三方备份继续使用其独立密码。",
  "vault.password.old": "旧密码",
  "vault.password.new": "新密码",
  "vault.password.confirm": "确认新密码",
  "vault.password.description":
    "修改密码将重新加密本机保存的主机、凭据、设置和历史。",
  "vault.password.success": "保险库密码已修改。",
  "vault.password.progress": "正在重新加密，请稍候…",
  "vault.password.submit": "确认修改",
  "vault.password.validation.old": "请输入旧密码，长度不能超过1024字节",
  "vault.password.validation.length":
    "新密码至少12个字符，长度不能超过1024字节",
  "vault.password.validation.unchanged": "新密码不能与旧密码相同",
  "vault.password.validation.mismatch": "两次输入的新密码须一致",
} as const;

export default {
  "account.oauth.confirmationCode": "浏览器一次性确认码",
  "account.oauth.codeDescription":
    "若浏览器未能打开 Termeta，请从授权成功页面复制一次性确认码，填入下方。",
  "account.oauth.manualFallback": "未自动返回？手动输入确认码",
  "account.oauth.returnHint":
    "如浏览器询问是否打开 Termeta，请允许；若未自动返回，可点击页面上的“返回 Termeta”。请保持此窗口打开，等待完成。",
  "account.oauth.codeSecurity":
    "仅填写本人在此设备发起的授权确认码；不要分享确认码或替他人填写。",
  "account.oauth.submitCode": "提交确认码",
  "account.oauth.codeSubmitted":
    "确认码已提交，正在等待验证。若未完成，请保留此窗口。",
  "account.oauth.validationCode":
    "请输入浏览器显示的 32 位一次性确认码，仅包含字母、数字、下划线或短横线。",

  "account.actions.clearLogin": "退出 / 清除本机登录",
  "account.title": "账号",
  "account.description":
    "登录官方 Termeta 账号，可使用邮箱密码、GitHub 或 Google。本地终端无需账号即可使用。",
  "account.encryptionHint":
    "账号登录不会解锁保险库，也不会替代保险库密码或备份加密密码。",
  "account.admin": "管理员",
  "account.createdAt": "注册时间：{date}",
  "account.expiresAt": "会话到期：{date}",
  "account.signedOut": "当前未登录。",
  "account.actions.refresh": "刷新账号状态",
  "account.actions.logout": "退出登录",
  "account.actions.login": "登录",
  "account.actions.register": "注册",
  "account.actions.retry": "重新读取账号",
  "account.actions.unlink": "解绑",
  "account.actions.linkProvider": "绑定 {provider}",
  "account.actions.loginProvider": "使用 {provider} 登录",
  "account.actions.retryProviders": "重新加载登录方式",
  "account.actions.cancelLogin": "取消等待",
  "account.actions.openSettings": "账号与设置",
  "account.identities.title": "登录方式",
  "account.identities.description":
    "绑定前请在浏览器确认第三方账号。相同邮箱不会自动合并账号。",
  "account.identities.password": "邮箱与密码",
  "account.identities.lastMethod": "至少需要保留一种登录方式。",
  "account.unlink.confirm":
    "确定解绑 {provider}？解绑后无法再通过该账号登录 Termeta。",
  "account.logout.confirm": "确定退出官方账号？本机保险库和终端会话不受影响。",
  "account.oauth.completed": "授权已完成，账号状态已更新。",
  "account.oauth.cancelled": "已取消本次浏览器授权。",
  "account.oauth.waiting":
    "请在系统浏览器中完成 {provider} 授权，随后将自动返回 Termeta 完成验证。",
  "account.oauth.deadline": "等待截止：{date}",
  "account.errors.expired": "授权已过期，请重新发起。",
  "account.errors.denied": "授权未完成，请重新发起或使用其他方式登录。",
  "account.form.email": "邮箱",
  "account.form.password": "密码",
  "account.form.confirmation": "确认密码",
  "account.validation.email": "请输入有效的邮箱地址。",
  "account.validation.password": "密码需为 12–256 个字符。",
  "account.validation.confirmation": "两次输入的密码不一致。",
} as const;

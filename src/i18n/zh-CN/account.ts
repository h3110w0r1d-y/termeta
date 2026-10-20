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
  "account.title": "账号",
  "account.description":
    "选择官方账号模式后，保险库会自动全量同步到 termeta.io；本地模式无需账号。",
  "account.actions.logout": "退出登录",
  "account.actions.loginProvider": "使用 {provider} 登录",
  "account.actions.retryProviders": "重新加载登录方式",
  "account.actions.cancelLogin": "取消等待",
  "account.actions.openSettings": "账号与设置",
  "account.logout.confirm":
    "确定退出官方账号并切换到本地模式？本机数据和终端会话将保留，官方自动同步将停止。",
  "account.oauth.waiting":
    "请在系统浏览器中完成 {provider} 授权，随后将自动返回 Termeta 完成验证。",
  "account.oauth.deadline": "等待截止：{date}",
  "account.errors.expired": "授权已过期，请重新发起。",
  "account.errors.denied": "授权未完成，请重新发起或使用其他方式登录。",
  "account.mode.title": "软件启动模式",
  "account.mode.description":
    "登录官方账号，将保险库自动同步到 termeta.io；或使用本地离线模式，自行管理数据。",
  "account.mode.official": "官方账号模式",
  "account.mode.local": "本地 / 离线模式",
  "account.mode.logoutLocal": "退出账号并使用本地模式",
  "account.browser.description":
    "账号注册、登录及登录方式管理均在浏览器中的 termeta.io 完成。",
  "account.sync.unlockTitle": "解锁云端保险库",
  "account.sync.unlockDescription":
    "云端已有数据。请输入其保险库密码，验证成功后合并本机数据并继续。密码错误时无法进入；可退出账号保留本机数据，改用本地模式。",
  "account.sync.setupDescription":
    "请设置保险库密码。此密码同时保护本机与官方云端数据，请牢记；账号登录无法恢复遗忘的保险库密码。",
  "account.sync.existingDescription":
    "请输入当前本机保险库密码，启用官方自动同步。后续官方云端与本机保险库使用同一个密码。",
  "account.sync.description":
    "官方云端自动全量同步已开启，使用与本机保险库相同的主密钥。退出账号会保留本机数据，并停止官方同步。",
  "account.sync.mainWindowRequired":
    "请回到 Termeta 主窗口输入保险库密码，完成云端解密与合并。验证完成前无法进入工作台。",
  "account.sync.lastSynced": "上次同步：{date}",
  "account.sync.now": "立即同步",
  "account.sync.continue": "验证并继续",
  "account.sync.localUnlockTitle": "先解锁本机保险库",
  "account.sync.localUnlockDescription":
    "请输入当前本机保险库密码，以便保留并合并本机数据。若云端密码已在其他设备更改，本机旧密码可能与云端不同；本机解锁后再验证云端密码，无需退出账号。",
} as const;

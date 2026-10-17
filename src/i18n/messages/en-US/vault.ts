// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "vault.validation.passwordRequired": "Enter your vault password",
  "vault.validation.passwordMismatch": "The passwords must match.",
  "vault.setup.title": "Create a vault password",
  "vault.unlock.title": "Unlock your vault",
  "vault.setup.method": "Create vault",
  "vault.setup.empty": "Create an empty vault",
  "vault.setup.cloud": "Restore a cloud backup",
  "vault.setup.passwordPlaceholder": "Password (at least 12 characters)",
  "vault.form.password": "Vault password",
  "vault.form.confirmPassword": "Confirm password",
  "vault.actions.createAndUnlock": "Create and unlock",
  "vault.actions.unlock": "Unlock",
  "vault.validation.passwordLength":
    "Use at least 12 characters and no more than 1024 bytes.",
  "vault.setup.warning":
    "Your hosts and credentials are encrypted on this device. Keep this password safe: if automatic unlock becomes unavailable and you forget it, your data cannot be recovered.",
  "vault.unlock.automaticProgress":
    "Trying automatic unlock. If it is unavailable, enter your vault password.",
  "vault.password.title": "Change vault password",
  "vault.password.warning":
    "Remember your new password. Existing local copies still require the old password. Your cloud backup password stays the same.",
  "vault.password.old": "Current password",
  "vault.password.new": "New password",
  "vault.password.confirm": "Confirm new password",
  "vault.password.description":
    "Changing your password re-encrypts the hosts, credentials, settings, and history saved on this device.",
  "vault.password.success": "Vault password changed.",
  "vault.password.progress": "Re-encrypting your data. Please wait…",
  "vault.password.submit": "Change password",
  "vault.password.validation.old":
    "Enter your current password (up to 1,024 bytes)",
  "vault.password.validation.length":
    "Use at least 12 characters and no more than 1,024 bytes",
  "vault.password.validation.unchanged":
    "Choose a password different from your current password",
  "vault.password.validation.mismatch": "The new passwords must match",
} as const;

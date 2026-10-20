export default {
  "account.oauth.confirmationCode": "One-time browser confirmation code",
  "account.oauth.codeDescription":
    "If your browser cannot open Termeta, copy the one-time confirmation code from the authorization success page and enter it below.",
  "account.oauth.manualFallback": "Not returning automatically? Enter a code",
  "account.oauth.returnHint":
    "Allow your browser to open Termeta when prompted. If it does not return automatically, select “Return to Termeta” on the page. Keep this window open until verification finishes.",
  "account.oauth.codeSecurity":
    "Only enter a code for authorization you started on this device. Never share the code or enter it for someone else.",
  "account.oauth.submitCode": "Submit confirmation code",
  "account.oauth.codeSubmitted":
    "Code submitted. Waiting for verification; keep this window open until it finishes.",
  "account.oauth.validationCode":
    "Enter the 32-character code from your browser, using only letters, digits, underscores, or hyphens.",
  "account.title": "Account",
  "account.description":
    "Official account mode automatically syncs your full vault to termeta.io. Local mode requires no account.",
  "account.actions.logout": "Sign out",
  "account.actions.loginProvider": "Continue with {provider}",
  "account.actions.retryProviders": "Reload sign-in methods",
  "account.actions.cancelLogin": "Cancel waiting",
  "account.actions.openSettings": "Account and settings",
  "account.logout.confirm":
    "Sign out and switch to local mode? Local data and terminal sessions will be kept. Automatic official sync will stop.",
  "account.oauth.waiting":
    "Complete {provider} authorization in your system browser. You will return to Termeta automatically to finish verification.",
  "account.oauth.deadline": "Waiting until: {date}",
  "account.errors.expired": "Authorization expired. Start again.",
  "account.errors.denied":
    "Authorization was not completed. Try again or use another sign-in method.",
  "account.mode.title": "Choose how to use Termeta",
  "account.mode.description":
    "Sign in to automatically sync your vault to termeta.io, or keep your data in local offline mode.",
  "account.mode.official": "Official account mode",
  "account.mode.local": "Local / offline mode",
  "account.mode.logoutLocal": "Sign out and use local mode",
  "account.browser.description":
    "Registration, sign-in, and sign-in methods are managed on termeta.io in your browser.",
  "account.sync.unlockTitle": "Unlock your cloud vault",
  "account.sync.unlockDescription":
    "Cloud data exists. Enter its vault password to verify, merge local data, and continue. An incorrect password prevents entry. You can sign out, keep local data, and use local mode.",
  "account.sync.setupDescription":
    "Set a vault password to protect both local and official cloud data. Remember it: signing in cannot recover a forgotten vault password.",
  "account.sync.existingDescription":
    "Enter your current local vault password to enable automatic official sync. Your local and official cloud vault will use the same password.",
  "account.sync.description":
    "Automatic full sync to the official cloud is enabled, using the same master key as your local vault. Signing out keeps local data and stops official sync.",
  "account.sync.mainWindowRequired":
    "Return to the main Termeta window and enter your vault password to decrypt and merge cloud data. The workbench stays locked until verification succeeds.",
  "account.sync.lastSynced": "Last synced: {date}",
  "account.sync.now": "Sync now",
  "account.sync.continue": "Verify and continue",
  "account.sync.localUnlockTitle": "Unlock your local vault first",
  "account.sync.localUnlockDescription":
    "Enter your current local vault password to preserve and merge local data. If the cloud password changed on another device, the old local password may differ. After unlocking locally, verify the cloud password without signing out.",
} as const;

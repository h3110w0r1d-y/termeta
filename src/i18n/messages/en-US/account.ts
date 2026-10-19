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

  "account.actions.clearLogin": "Sign out / clear local sign-in",
  "account.title": "Account",
  "account.description":
    "Sign in to your official Termeta account with email and password, GitHub, or Google. Local terminals work without an account.",
  "account.encryptionHint":
    "Signing in does not unlock your vault or replace your vault password or backup encryption password.",
  "account.admin": "Administrator",
  "account.createdAt": "Registered: {date}",
  "account.expiresAt": "Session expires: {date}",
  "account.signedOut": "You are signed out.",
  "account.actions.refresh": "Refresh account",
  "account.actions.logout": "Sign out",
  "account.actions.login": "Sign in",
  "account.actions.register": "Register",
  "account.actions.retry": "Reload account",
  "account.actions.unlink": "Unlink",
  "account.actions.linkProvider": "Link {provider}",
  "account.actions.loginProvider": "Continue with {provider}",
  "account.actions.retryProviders": "Reload sign-in methods",
  "account.actions.cancelLogin": "Cancel waiting",
  "account.actions.openSettings": "Account and settings",
  "account.identities.title": "Sign-in methods",
  "account.identities.description":
    "Confirm the provider account in your browser before linking. Matching email addresses never merge accounts automatically.",
  "account.identities.password": "Email and password",
  "account.identities.lastMethod": "Keep at least one sign-in method.",
  "account.unlink.confirm":
    "Unlink {provider}? You will no longer be able to use that account to sign in to Termeta.",
  "account.logout.confirm":
    "Sign out of your official account? Your local vault and terminal sessions will remain available.",
  "account.oauth.completed":
    "Authorization completed. Your account has been updated.",
  "account.oauth.cancelled": "Browser authorization cancelled.",
  "account.oauth.waiting":
    "Complete {provider} authorization in your system browser. You will return to Termeta automatically to finish verification.",
  "account.oauth.deadline": "Waiting until: {date}",
  "account.errors.expired": "Authorization expired. Start again.",
  "account.errors.denied":
    "Authorization was not completed. Try again or use another sign-in method.",
  "account.form.email": "Email",
  "account.form.password": "Password",
  "account.form.confirmation": "Confirm password",
  "account.validation.email": "Enter a valid email address.",
  "account.validation.password": "Use a password with 12–256 characters.",
  "account.validation.confirmation": "The passwords do not match.",
} as const;

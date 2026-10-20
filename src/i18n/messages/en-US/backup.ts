// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "backup.title": "Backup and sync",
  "backup.form.provider": "Backup service",
  "backup.form.deviceName": "Device name",
  "backup.form.devicePlaceholder": "For example: My MacBook",
  "backup.form.webdavUrl": "WebDAV folder URL (HTTPS)",
  "backup.form.password": "Backup password (at least 12 characters)",
  "backup.form.confirmPassword": "Confirm backup password",
  "backup.form.operation": "Backup or sync",
  "backup.empty": "No backups found in this folder or account.",
  "backup.import.title": "Confirm cloud import",
  "backup.description":
    "Back up hosts, groups, credentials, forwarding rules, and trusted host keys with encryption. App settings stay on this device. Keep your backup password safe.",
  "backup.passwordHint":
    "Your backup password is saved on this device. Changing it affects new backups only. Older backups still require their original password.",
  "backup.actions.test": "Test connection",
  "backup.actions.saveConfig": "Save cloud settings",
  "backup.actions.uploadVersion": "Back up · Upload a new version",
  "backup.actions.downloadVersion": "Sync · Download from cloud",
  "backup.actions.preview": "Download and preview",
  "backup.actions.loadMore": "Load more backups",
  "backup.form.githubToken": "GitHub token (Gist permission required)",
  "backup.form.webdavPassword": "WebDAV password / App password",
  "backup.form.savedSecretPlaceholder":
    "Saved. Leave blank to keep it or enter a replacement",
  "backup.form.credentialsHint": "Credentials are encrypted on this device",
  "backup.form.passwordHint":
    "Your backup password will be encrypted on this device",
  "backup.version.undecryptable": "Unable to decrypt backup",
  "backup.version.decryptError":
    "The password is incorrect or the backup is damaged.",
  "backup.upload.description": "Each upload creates a new backup version.",
  "backup.download.description":
    "Choose a version, preview it, and confirm the import.",
  "backup.upload.progress": "Encrypt and upload backup",
  "backup.download.progress": "Load cloud backups",
  "backup.import.mergeDescription":
    "Imported data is added alongside your local data and settings. Importing again creates duplicates. Conflicting host keys are skipped with a warning.",
  "backup.actions.confirmImport": "Import as new data",
  "backup.import.counts":
    "Hosts {hosts} · Groups {groups} · Credentials {credentials} · Forwards {forwards} · Trusted keys {knownHosts} · Snippets {snippets}",
  "backup.import.success":
    "Sync complete: {counts}. Existing local data and settings were kept.",
  "backup.defaultDeviceName": "My device",
  "backup.validation.deviceName": "Enter a device name (up to 256 bytes)",
  "backup.validation.urlFormat":
    "Enter an HTTPS folder URL without credentials, query parameters, or fragments",
  "backup.validation.webdavUrl": "Enter a valid WebDAV HTTPS folder URL",
  "backup.validation.username":
    "Enter a username without colons (up to 1,024 bytes)",
  "backup.validation.credentials": "Enter your cloud credentials",
  "backup.validation.passwordRequired": "Enter a backup password",
  "backup.validation.passwordMaxLength":
    "The password must not exceed 1,024 bytes",
  "backup.validation.passwordMinLength":
    "Use at least 12 characters for your backup password",
  "backup.validation.passwordMismatch": "The backup passwords must match",
  "backup.status.configSaved":
    "Cloud settings were encrypted and saved on this device.",
  "backup.status.connected": "Connected successfully.",
  "backup.status.uploaded":
    "New backup uploaded. Existing cloud backups were kept.",
} as const;

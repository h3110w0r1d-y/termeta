// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "credentials.validation.passwordRequired": "Enter a password",
  "credentials.confirmDelete":
    "Delete credential “{label}” from your vault? This cannot be undone.",
  "credentials.description":
    "Save passwords and private keys to reuse across hosts.",
  "credentials.actions.new": "New credential",
  "credentials.search.label": "Search credentials",
  "credentials.search.placeholder": "Search credential names…",
  "credentials.status.loading": "Loading credentials…",
  "credentials.search.noResults":
    "No matching credentials. Try another search.",
  "credentials.empty":
    "No saved credentials yet. Create one to use when connecting to hosts.",
  "credentials.actions.create": "Create credential",
  "credentials.types.password": "Password",
  "credentials.types.privateKey": "Private key",
  "credentials.status.ready": "Ready",
  "credentials.status.awaitingRecovery": "Awaiting recovery",
  "credentials.status.pendingDeletion": "Pending deletion",
  "credentials.delete.referenceHint":
    "Remove this credential from host settings first",
  "credentials.actions.delete": "Delete credential",
  "credentials.actions.retryDelete": "Retry deletion",
  "credentials.form.description":
    "Saved securely in your vault for host authentication.",
  "credentials.form.namePlaceholder": "e.g. Production SSH password",
  "credentials.form.sshPassword": "SSH password",
  "credentials.form.sshPrivateKey": "SSH private key",
  "credentials.form.privateKeyPlaceholder":
    "Paste the complete OpenSSH private key, including the BEGIN and END lines",
  "credentials.form.passphrase": "Key passphrase (optional)",
  "credentials.form.passphraseHint":
    "Leave blank to be asked when connecting, if needed",
  "credentials.actions.save": "Save credential",
  "credentials.delete.description":
    "Remove credentials from host settings before deleting them.",
  "credentials.form.keyHint":
    "The private key will be encrypted and saved in your vault. Maximum size: 1 MiB.",
  "credentials.referenceCount": "Used by {count} hosts",
  "credentials.actions.showPublicKey": "View public key",
  "credentials.actions.copyPublicKey": "Copy public key",
  "credentials.publicKey.title": "SSH public key",
  "credentials.publicKey.loading": "Loading public key…",
  "credentials.publicKey.hint":
    "You can add this key to the server’s authorized_keys file.",
  "credentials.publicKey.copied": "Public key copied",
} as const;

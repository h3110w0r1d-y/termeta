// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "forwarding.validation.nameLength": "Name must be at most 128 characters",
  "forwarding.form.bindAddress": "Listen IP address",
  "forwarding.form.bindPort": "Listen port",
  "forwarding.form.targetAddress": "Target address",
  "forwarding.form.targetPort": "Target port",
  "forwarding.actions.edit": "Edit port forwarding",
  "forwarding.actions.new": "New port forwarding",
  "forwarding.form.namePlaceholder":
    "e.g. Database, development service or proxy",
  "forwarding.form.hostHint": "Create an SSH host on the Hosts page first.",
  "forwarding.form.mode": "Forwarding mode",
  "forwarding.modes.local": "Local forwarding",
  "forwarding.modes.remote": "Remote forwarding",
  "forwarding.modes.dynamic": "Dynamic SOCKS5",
  "forwarding.form.remoteBindAddress": "Remote listen IP",
  "forwarding.form.localBindAddress": "Local listen IP",
  "forwarding.form.loopbackPlaceholder": "127.0.0.1 or ::1",
  "forwarding.form.localTargetAddress": "Local target address",
  "forwarding.form.remoteTargetAddress": "Remote target address",
  "forwarding.form.targetPlaceholder": "Hostname or IP; omit brackets for IPv6",
  "forwarding.confirmDelete":
    "Delete port forwarding “{name}”? The SSH host will be kept.",
  "forwarding.actions.create": "New forwarding",
  "forwarding.search.placeholder":
    "Search name, SSH host or forwarding address",
  "forwarding.search.label": "Search port forwarding",
  "forwarding.search.noResults": "No matching forwarding rules",
  "forwarding.empty": "No forwarding rules yet",
  "forwarding.status.stopped": "Stopped",
  "forwarding.errors.hostDeleted": "The SSH host has been deleted",
  "forwarding.endpoints.local": "Local",
  "forwarding.endpoints.dynamicTarget": "SOCKS5 → Target chosen by application",
  "forwarding.endpoints.localSide": "Local",
  "forwarding.endpoints.remoteSide": "Remote",
  "forwarding.endpoints.target": "→ {value1} {value2}",
  "forwarding.descriptions.local":
    "Access a remote target through a local port. The SSH server resolves the target address.",
  "forwarding.descriptions.remote":
    "Access a local-side target through a server port. Your computer resolves the target address.",
  "forwarding.descriptions.dynamic":
    "Create a SOCKS5 proxy available only on this computer. The SSH server resolves hostnames. TCP only, without authentication.",
  "forwarding.warning.publicListener":
    "Listening on 0.0.0.0 or :: allows other devices to connect. Remote access also depends on server settings.",
} as const;

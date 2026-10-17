import { t } from "../../i18n";
import type { Locale } from "../../i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { Channel, invoke } from "@tauri-apps/api/core";
import type { AppSettings, SettingsPatch } from "../../modules/settings/model";
import type {
  BackupConfigDraft,
  BackupConfigView,
  BackupImportSummary,
  BackupListEntry,
  BackupRestorePreview,
  VaultStatus,
  PasswordChangeResult,
  ForwardDraft,
  ForwardProfile,
  DirectoryPage,
  SftpCopyRequest,
  SftpLocalDrop,
  SftpDropPosition,
  TransferProgress,
  AuthChallenge,
  CredentialMetadata,
  CredentialDraft,
  Snippet,
  SnippetDraft,
  SessionLog,
  SessionLogDraft,
  SessionLogSummary,
  HostDraft,
  WorkspaceDraft,
  HostGroup,
  HostProfile,
  KnownHost,
  OpenTarget,
  SessionEvent,
  SessionSnapshot,
  SerialPortInfo,
  TerminalSize,
} from "./types";

// 原生交互只通过此门面，组件不能获得通用 invoke 或任意文件读取能力。
export const desktop = {
  getBackupConfig: () => invoke<BackupConfigView | null>("get_backup_config"),
  saveBackupConfig: (config: BackupConfigDraft) =>
    invoke<BackupConfigView>("save_backup_config", { config }),
  listCloudBackups: (
    config?: BackupConfigDraft,
    initializing = false,
    offset = 0,
    limit = 10,
  ) =>
    invoke<BackupListEntry[]>("list_cloud_backups", {
      config,
      initializing,
      offset,
      limit,
    }),
  testBackupConfig: (config: BackupConfigDraft, initializing = false) =>
    invoke<void>("test_backup_config", { config, initializing }),
  createCloudBackup: (deviceName?: string) =>
    invoke<BackupListEntry>("create_cloud_backup", { deviceName }),
  prepareCloudRestore: (
    backupId: string,
    config?: BackupConfigDraft,
    initializing = false,
  ) =>
    invoke<BackupRestorePreview>("prepare_cloud_restore", {
      config,
      backupId,
      initializing,
    }),
  applyCloudRestore: (token: string) =>
    invoke<BackupImportSummary>("apply_cloud_restore", { token }),
  initializeFromCloudBackup: (token: string, password: string) =>
    invoke<VaultStatus>("initialize_from_cloud_backup", { token, password }),
  onBackupRestored: (handler: () => void) => listen("backup-restored", handler),
  saveWorkspace: (draft: WorkspaceDraft) =>
    invoke<HostProfile>("save_workspace", { draft }),
  listPortForwards: () =>
    invoke<{ profiles: ForwardProfile[]; sessions: SessionSnapshot[] }>(
      "list_port_forwards",
    ),
  savePortForward: (draft: ForwardDraft, expectedRevision: number | null) =>
    invoke<ForwardProfile>("save_port_forward", { draft, expectedRevision }),
  deletePortForward: (id: string, revision: number) =>
    invoke<void>("delete_port_forward", { id, revision }),
  startPortForward(
    id: string,
    requestId: string,
    control: (value: SessionEvent) => void,
  ) {
    const onControl = new Channel<SessionEvent>();
    onControl.onmessage = control;
    return invoke<SessionSnapshot>("start_port_forward", {
      id,
      requestId,
      onControl,
    });
  },
  stopPortForward: (id: string) => invoke<void>("stop_port_forward", { id }),
  listSerialPorts: () => invoke<SerialPortInfo[]>("list_serial_ports"),
  vaultStatus: () => invoke<VaultStatus>("vault_status"),
  unlockVault: (password: string, setup: boolean) =>
    invoke<VaultStatus>("unlock_vault", { password, setup }),
  changeVaultPassword: (oldPassword: string, newPassword: string) =>
    invoke<PasswordChangeResult>("change_vault_password", {
      oldPassword,
      newPassword,
    }),
  sftpConnect(
    profileId: string,
    requestId: string,
    control: (value: SessionEvent) => void,
  ) {
    const onControl = new Channel<SessionEvent>();
    onControl.onmessage = control;
    return invoke<SessionSnapshot>("sftp_connect", {
      profileId,
      requestId,
      onControl,
    });
  },
  sftpCopy(
    request: SftpCopyRequest,
    progress: (value: TransferProgress) => void,
  ) {
    const onProgress = new Channel<TransferProgress>();
    onProgress.onmessage = progress;
    return invoke<void>("sftp_copy", { request, onProgress });
  },
  sftpList: (sessionId: string, path: string, more = false) =>
    invoke<DirectoryPage>("sftp_list", { sessionId, path, more }),
  sftpTransfer(
    sessionId: string,
    transferId: string,
    directory: string,
    name: string | null,
    upload: boolean,
    progress: (value: TransferProgress) => void,
  ) {
    const onProgress = new Channel<TransferProgress>();
    onProgress.onmessage = progress;
    return invoke<boolean>("sftp_transfer", {
      sessionId,
      transferId,
      directory,
      name,
      upload,
      onProgress,
    });
  },
  sftpDropUpload(
    sessionId: string,
    transferId: string,
    dropId: string,
    directory: string,
    progress: (value: TransferProgress) => void,
  ) {
    const onProgress = new Channel<TransferProgress>();
    onProgress.onmessage = progress;
    return invoke<boolean>("sftp_drop_upload", {
      sessionId,
      transferId,
      dropId,
      directory,
      onProgress,
    });
  },
  onSftpLocalHover: (callback: (position: SftpDropPosition) => void) =>
    listen<{ position: SftpDropPosition }>("sftp-local-hover", (event) =>
      callback(event.payload.position),
    ),
  onSftpLocalDrop: (callback: (drop: SftpLocalDrop) => void) =>
    listen<SftpLocalDrop>("sftp-local-drop", (event) =>
      callback(event.payload),
    ),
  onSftpLocalLeave: (callback: () => void) =>
    listen("sftp-local-leave", callback),
  sftpStartDrag(
    sessionId: string,
    transferId: string,
    directory: string,
    names: string[],
    directories: string[],
    progress: (value: TransferProgress) => void,
  ) {
    const onProgress = new Channel<TransferProgress>();
    onProgress.onmessage = progress;
    return invoke<boolean>("sftp_start_drag", {
      sessionId,
      transferId,
      directory,
      names,
      directories,
      onProgress,
    });
  },
  sftpCancel: (sessionId: string, transferId: string) =>
    invoke<void>("sftp_cancel", { sessionId, transferId }),
  exportJson: (name: string, value: unknown) =>
    invoke<boolean>("export_json", { name, value }),
  onCloseRequested: (
    callback: Parameters<
      ReturnType<typeof getCurrentWindow>["onCloseRequested"]
    >[0],
  ) => getCurrentWindow().onCloseRequested(callback),
  closeWindow: () => getCurrentWindow().close(),
  onAppExitRequested: (handler: () => void) =>
    listen("app-exit-requested", handler),
  finishAppExit: () => invoke<void>("finish_app_exit"),
  setWindowTitle: (title: string) => getCurrentWindow().setTitle(title),
  setNativeMenuLocale: (locale: Locale) =>
    invoke<void>("set_native_menu_locale", { locale }),
  minimizeWindow: () => getCurrentWindow().minimize(),
  toggleMaximizeWindow: () => getCurrentWindow().toggleMaximize(),
  listHosts: () => invoke<HostProfile[]>("list_hosts"),
  saveHost: (
    host: HostDraft,
    expectedRevision: number | null,
    password?: string,
  ) => invoke<HostProfile>("save_host", { host, expectedRevision, password }),
  deleteHost: (id: string, revision: number) =>
    invoke<void>("delete_host", { id, revision }),
  listGroups: () => invoke<HostGroup[]>("list_host_groups"),
  saveGroup: (
    group: Omit<HostGroup, "id" | "revision"> & { id: string | null },
    expectedRevision: number | null,
  ) => invoke<HostGroup>("save_host_group", { group, expectedRevision }),
  deleteGroup: (id: string, revision: number, deleteHosts = false) =>
    invoke<void>("delete_host_group", { id, revision, deleteHosts }),
  isSettingsWindow: () => getCurrentWindow().label === "settings",
  openSettingsWindow: () => invoke<void>("open_settings_window"),
  onSettingsChanged: (handler: () => void) =>
    listen("settings-changed", handler),
  getSettings: () => invoke<AppSettings | null>("get_settings"),
  saveSettings: (settings: SettingsPatch, defaults: AppSettings) =>
    invoke<AppSettings>("save_settings", { settings, defaults }),
  selectKey: () =>
    invoke<{ keyFileRef: string; name: string } | null>(
      "select_private_key_file",
    ),
  listKnownHosts: () => invoke<KnownHost[]>("list_known_hosts"),
  revokeKnownHost: (id: string) => invoke<void>("revoke_known_host", { id }),
  saveCredential: (payload: CredentialDraft) =>
    invoke<{ id: string }>("save_credential", { ...payload }),
  listSnippets: () => invoke<Snippet[]>("list_snippets"),
  saveSnippet: (draft: SnippetDraft) =>
    invoke<Snippet>("save_snippet", { draft }),
  deleteSnippet: (id: string, revision: number) =>
    invoke<void>("delete_snippet", { id, revision }),
  saveSessionLog: (draft: SessionLogDraft) =>
    invoke<SessionLogSummary>("save_session_log", { draft }),
  listSessionLogs: () => invoke<SessionLogSummary[]>("list_session_logs"),
  readSessionLog: (id: string) =>
    invoke<SessionLog>("read_session_log", { id }),
  deleteSessionLog: (id: string) => invoke<void>("delete_session_log", { id }),
  clearSessionLogs: () => invoke<void>("clear_session_logs"),
  listCredentials: () => invoke<CredentialMetadata[]>("list_credentials"),
  deleteCredential: (id: string) => invoke<void>("delete_credential", { id }),
  openUrl: (url: string) => invoke<void>("open_external_url", { url }),
  openSession(
    terminalId: string,
    target: OpenTarget,
    size: TerminalSize,
    onOutput: (data: ArrayBuffer) => void,
    onControl: (event: SessionEvent) => void,
  ) {
    const output = new Channel<ArrayBuffer>();
    const control = new Channel<SessionEvent>();
    output.onmessage = onOutput;
    control.onmessage = onControl;
    return invoke<SessionSnapshot>("open_session", {
      protocolVersion: 1,
      requestId: crypto.randomUUID(),
      terminalId,
      target,
      size,
      onOutput: output,
      onControl: control,
    });
  },
  write: (sessionId: string, inputSeq: number, bytes: number[]) =>
    invoke("write_session", { sessionId, inputSeq, bytes }),
  resize: (sessionId: string, size: TerminalSize) =>
    invoke("resize_session", { sessionId, size }),
  ack: (sessionId: string, consumedOffset: string) =>
    invoke<void>("ack_session_output", { sessionId, consumedOffset }),
  close: (sessionId: string, reason = "user") =>
    invoke<SessionSnapshot>("close_session", { sessionId, reason }),
  hostKey: (
    sessionId: string,
    challengeId: string,
    decision: "reject" | "allowOnce" | "trustAndSave",
  ) => invoke<void>("respond_host_key", { sessionId, challengeId, decision }),
  auth: (
    challenge: AuthChallenge,
    answers: string[],
    savePolicy: "never" | "save",
  ) =>
    invoke<void>("respond_auth_challenge", {
      sessionId: challenge.sessionId,
      challengeId: challenge.challengeId,
      answers,
      savePolicy,
    }),
  cancelAuth: (sessionId: string, challengeId: string) =>
    invoke<void>("cancel_auth_challenge", { sessionId, challengeId }),
  heartbeat: () => invoke<void>("session_client_heartbeat"),
};
export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error)
    return String(error.message);
  return t("common.errors.operationFailed");
}

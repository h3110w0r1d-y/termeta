export type SessionState =
  | "created"
  | "connecting"
  | "awaitingHostKey"
  | "authenticating"
  | "awaitingAuth"
  | "opening"
  | "connected"
  | "closing"
  | "closed"
  | "failed"
  | "disconnected";
export interface TerminalSize {
  cols: number;
  rows: number;
}
export type AuthStep =
  | { type: "password"; credentialId?: string }
  | { type: "privateKey"; credentialId: string };
export type SshStartup = { type: "shell" } | { type: "exec"; command: string };
export interface SshConfig {
  configVersion: 1;
  address: string;
  port: number;
  username: string;
  authPlan: AuthStep[];
  startup?: SshStartup;
}
export interface TelnetConfig {
  configVersion: 1;
  address: string;
  port: number;
}
export type HostKind = "ssh" | "serial" | "telnet";
export interface HostProfile extends Omit<SshConfig, "configVersion"> {
  id: string;
  kind: HostKind | "workspace";
  serialConfig?: SerialConfig | null;
  workspace?: SavedWorkspace | null;
  name: string;
  groupId: string | null;
  favorite: boolean;
  tags: string[];
  revision: number;
  hasPassword: boolean;
  icon: string;
  systemDetectionAttempted: boolean;
}
export type HostDraft = Omit<
  HostProfile,
  "id" | "revision" | "hasPassword" | "systemDetectionAttempted"
> & {
  id: string | null;
};
export type SavedLayout =
  | { kind: "connection"; profileId: string }
  | {
      kind: "split";
      axis: "horizontal" | "vertical";
      ratio: number;
      first: SavedLayout;
      second: SavedLayout;
    };
export interface SavedWorkspace {
  root: SavedLayout;
  focusedIndex: number;
}
export interface WorkspaceDraft {
  name: string;
  workspace: SavedWorkspace;
  expectedRevision: number | null;
}
export interface HostGroup {
  id: string;
  parentId: string | null;
  name: string;
  sortOrder: number;
  revision: number;
}
export interface SerialConfig {
  path: string;
  baudRate: number;
  charset: string;
  dataBits: 5 | 6 | 7 | 8;
  stopBits: 1 | 2;
  flowControl: "none" | "hardware" | "software";
  parity: "none" | "odd" | "even";
}
export interface SerialPortInfo {
  path: string;
  description: string | null;
}
export type OpenTarget =
  | { source: "local" }
  | { source: "serial"; config: SerialConfig }
  | { source: "telnet"; config: TelnetConfig }
  | { source: "saved"; profileId: string }
  | { source: "ephemeral"; kind: "ssh"; config: SshConfig };
export interface SessionSnapshot {
  sshCommand: boolean;
  protocolVersion: 1;
  requestId: string;
  sessionId: string;
  kind: "ssh" | "local" | "serial" | "telnet" | "forwarding";
  terminalId: string;
  capabilities: {
    resize: boolean;
    pty: boolean;
    binaryOutput: boolean;
    serialControl: boolean;
  };
  title: string;
  state: SessionState;
  stateRevision: number;
  size: TerminalSize;
}
export type SessionPresenceKind = "terminal" | "file" | "forward";
export interface SessionPresenceRef {
  kind: SessionPresenceKind;
  sessionId: string;
}
export interface SessionPresenceHeartbeat {
  protocolVersion: 1;
  sessions: SessionPresenceRef[];
}
export type ForwardMode = "local" | "remote" | "dynamic";
export interface ForwardRule {
  mode: ForwardMode;
  bindAddress: string;
  bindPort: number;
  targetAddress: string | null;
  targetPort: number | null;
}
export interface ForwardProfile {
  id: string;
  name: string;
  profileId: string | null;
  rule: ForwardRule;
  revision: number;
}
export interface ForwardDraft {
  id: string | null;
  name: string;
  profileId: string;
  rule: ForwardRule;
}
export interface HostKeyChallenge {
  type: "hostKeyChallenge";
  sessionId: string;
  challengeId: string;
  endpoint: { host: string; canonicalHost: string; port: number };
  algorithm: string;
  fingerprint: string;
  expiresAt?: number;
}
export interface AuthChallenge {
  type: "authChallenge";
  sessionId: string;
  challengeId: string;
  name: string;
  instructions: string;
  prompts: { text: string; echo: boolean }[];
  expiresAt?: number;
}
export type SessionEvent =
  | { type: "hostSystemChanged"; sessionId: string; profileId: string }
  | HostKeyChallenge
  | AuthChallenge
  | { type: "stateChanged"; sessionId: string; snapshot: SessionSnapshot }
  | {
      type: "notice";
      sessionId: string;
      code: string;
      messageKey: string;
      details?: string;
    }
  | {
      type: "ended";
      sessionId: string;
      finalState: SessionState;
      reason: string;
      finalOffset: string;
      outputComplete: boolean;
      exitCode?: number | null;
      exitSignal?: string | null;
      error?: { message?: string; code?: string; details?: string };
    };
export interface KnownHost {
  id: string;
  canonicalHost: string;
  port: number;
  keyAlgorithm: string;
  fingerprintSha256: string;
  trustedAt: number;
}
export interface Snippet {
  id: string;
  name: string;
  content: string;
  revision: number;
  createdAt: number;
  updatedAt: number;
}
export interface SnippetDraft {
  id?: string;
  revision?: number;
  name: string;
  content: string;
}
export interface SessionLogDraft {
  id: string;
  protocol: "ssh" | "telnet" | "serial" | "local";
  name: string;
  username?: string | null;
  host?: string | null;
  port?: number | null;
  startedAt: number;
  content: string;
}
export interface SessionLogSummary extends Omit<SessionLogDraft, "content"> {
  closedAt: number;
  byteLength: number;
}
export interface SessionLog extends SessionLogSummary {
  content: string;
}
export interface CredentialMetadata {
  status: string;
  id: string;
  label: string;
  kind: "password" | "privateKey";
}
export interface CredentialDraft {
  kind: "password" | "privateKey";
  label: string;
  password?: string;
  keyFileRef?: string;
  passphrase?: string;
}

export interface FileEntry {
  name: string;
  kind: "directory" | "file" | "symlink" | "other";
  linkTarget: string | null;
  targetKind: "directory" | "file" | "other" | null;
  size: string | null;
  modified: number | null;
  permissions: number | null;
}
export interface DirectoryPage {
  path: string;
  // 每页最多100个远端条目，..导航行由视图独立生成。
  entries: FileEntry[];
  hasMore: boolean;
}
// 原生桥归一后的WebView逻辑坐标。
export interface SftpDropPosition {
  x: number;
  y: number;
}
export interface SftpLocalDrop {
  error?: string;
  dropId: string;
  position: SftpDropPosition;
}
export interface TransferProgress {
  currentName?: string;
  completedFiles?: number;
  totalFiles?: number;
  transferred: string;
  total: string | null;
}

export interface SftpCopyRequest {
  sourceSessionId: string;
  targetSessionId: string;
  sourcePath: string;
  targetPath: string;
  names: string[];
  transferId: string;
}

export interface VaultStatus {
  mode: "setup" | "locked" | "unlocked";
  keychainWarning: string | null;
  cleanupWarning: string | null;
}

export interface PasswordChangeResult {
  keychainWarning: string | null;
  cleanupWarning: string | null;
}

export type BackupProvider = "gist" | "webdav";
export interface BackupConfigDraft {
  provider: BackupProvider;
  endpoint: string;
  username: string;
  secret?: string;
  backupPassword?: string;
  deviceName: string;
}
export interface BackupConfigView extends Omit<
  BackupConfigDraft,
  "secret" | "backupPassword"
> {
  hasSecret: boolean;
  hasBackupPassword: boolean;
}
export interface BackupCounts {
  snippets: number;
  hosts: number;
  groups: number;
  credentials: number;
  forwards: number;
  knownHosts: number;
}
export interface BackupImportSummary {
  counts: BackupCounts;
  warnings: string[];
}
export interface BackupMetadata {
  backupId: string;
  createdAt: number;
  deviceName: string;
  appVersion: string;
  counts: BackupCounts;
}
export interface BackupListEntry {
  id: string;
  metadata: BackupMetadata | null;
  error?: string | null;
  size?: number | null;
}
export interface BackupRestorePreview {
  token: string;
  metadata: BackupMetadata;
}

export type AccountProviderId = "github" | "google";
export interface AccountUser {
  id: string;
  email: string;
  isAdmin: boolean;
  hasPassword: boolean;
  createdAt: string;
}
export interface AccountStatus {
  user: AccountUser | null;
  expiresAt: string | null;
}
export interface AccountProvider {
  id: AccountProviderId;
  name: string;
}
export interface AccountIdentity {
  provider: AccountProviderId;
  createdAt: string;
}
export interface AccountOAuthFlow {
  flowId: string;
  provider: AccountProviderId;
  expiresAt: string;
  pollInterval: number;
}
export interface AccountOAuthResult {
  status: "pending" | "completed" | "denied" | "expired";
  account: AccountStatus | null;
  error: string | null;
}

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
export interface SshConfig {
  configVersion: 1;
  address: string;
  port: number;
  username: string;
  authPlan: AuthStep[];
}
export interface HostProfile extends Omit<SshConfig, "configVersion"> {
  id: string;
  name: string;
  groupId: string | null;
  favorite: boolean;
  tags: string[];
  revision: number;
  hasPassword: boolean;
}
export type HostDraft = Omit<HostProfile, "id" | "revision" | "hasPassword"> & {
  id: string | null;
};
export interface HostGroup {
  id: string;
  parentId: string | null;
  name: string;
  sortOrder: number;
  revision: number;
}
export type OpenTarget =
  | { source: "saved"; profileId: string }
  | { source: "ephemeral"; kind: "ssh"; config: SshConfig };
export interface SessionSnapshot {
  protocolVersion: 1;
  requestId: string;
  sessionId: string;
  kind: "ssh";
  title: string;
  state: SessionState;
  stateRevision: number;
  size: TerminalSize;
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
      exitCode?: number;
      error?: { message?: string; code?: string };
    };
export interface KnownHost {
  id: string;
  canonicalHost: string;
  port: number;
  keyAlgorithm: string;
  fingerprintSha256: string;
  trustedAt: number;
}
export interface HistoryEntry {
  id: string;
  profileId: string | null;
  displayName: string;
  endpoint: string;
  startedAt: number;
  endedAt: number | null;
  endReason: string | null;
  errorCode: string | null;
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

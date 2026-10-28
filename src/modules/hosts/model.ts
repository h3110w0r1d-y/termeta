import type {
  SerialConfig,
  SshConfig,
  TelnetConfig,
} from "@/services/desktop/types";
import type { HostGroup, SavedWorkspace } from "@/services/desktop/types";

export type {
  HostGroup,
  SavedLayout,
  SavedWorkspace,
  WorkspaceDraft,
} from "@/services/desktop/types";
export type HostKind = "ssh" | "serial" | "telnet";

export interface HostMetadata {
  id: string;
  name: string;
  groupId: string | null;
  tags: string[];
  icon: string;
  sortOrder: number;
  lastUsedAt: number | null;
  revision: number;
}
export interface SshHostProfile
  extends HostMetadata, Omit<SshConfig, "configVersion"> {
  kind: "ssh";
  hasPassword: boolean;
  systemDetectionAttempted: boolean;
}
export interface TelnetHostProfile
  extends HostMetadata, Omit<TelnetConfig, "configVersion"> {
  kind: "telnet";
}
export interface SerialHostProfile extends HostMetadata {
  kind: "serial";
  serialConfig: SerialConfig;
}
export interface WorkspaceProfile extends HostMetadata {
  kind: "workspace";
  workspace: SavedWorkspace;
}
export type ConnectionProfile =
  SshHostProfile | TelnetHostProfile | SerialHostProfile;
export type HostProfile = ConnectionProfile | WorkspaceProfile;
export type NetworkHostProfile = SshHostProfile | TelnetHostProfile;

type DraftMetadata = Pick<
  HostMetadata,
  "name" | "groupId" | "tags" | "icon"
> & { id: string | null };
export type SshHostDraft = DraftMetadata &
  Omit<SshConfig, "configVersion"> & { kind: "ssh" };
export type TelnetHostDraft = DraftMetadata &
  Omit<TelnetConfig, "configVersion"> & { kind: "telnet" };
export type SerialHostDraft = DraftMetadata & {
  kind: "serial";
  serialConfig: SerialConfig;
};
export type HostDraft = SshHostDraft | TelnetHostDraft | SerialHostDraft;
export interface HostInventory {
  hosts: HostProfile[];
  groups: HostGroup[];
}

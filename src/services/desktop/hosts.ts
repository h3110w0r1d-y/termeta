import { t } from "@/i18n";
import type {
  HostDraft,
  HostInventory,
  HostProfile,
} from "@/modules/hosts/model";
import type { HostDraftDto, HostInventoryDto, HostProfileDto } from "./types";

/** 扁平 wire 中仅有占位含义的字段在此止步，业务必须先按 kind 收窄。 */
export function hostProfileFromDto(dto: HostProfileDto): HostProfile {
  const { id, name, groupId, tags, icon, sortOrder, lastUsedAt, revision } =
    dto;
  const metadata = {
    id,
    name,
    groupId,
    tags,
    icon,
    sortOrder,
    lastUsedAt,
    revision,
  };
  const invalid = () => new Error(t("hosts.errors.invalidConfiguration"));
  if (
    dto.kind !== "ssh" &&
    ((dto.startup && dto.startup.type !== "shell") ||
      dto.username ||
      dto.authPlan.length ||
      dto.hasPassword)
  )
    throw invalid();
  if (dto.kind !== "serial" && dto.serialConfig != null) throw invalid();
  if (dto.kind !== "workspace" && dto.workspace != null) throw invalid();
  switch (dto.kind) {
    case "ssh":
      return {
        ...metadata,
        kind: "ssh",
        address: dto.address,
        port: dto.port,
        username: dto.username,
        authPlan: dto.authPlan,
        startup: dto.startup,
        hasPassword: dto.hasPassword,
        systemDetectionAttempted: dto.systemDetectionAttempted,
      };
    case "telnet":
      return {
        ...metadata,
        kind: "telnet",
        address: dto.address,
        port: dto.port,
      };
    case "serial":
      if (!dto.serialConfig)
        throw new Error(t("serial.errors.missingConfiguration"));
      if (dto.address || dto.port !== 0) throw invalid();
      return { ...metadata, kind: "serial", serialConfig: dto.serialConfig };
    case "workspace":
      if (!dto.workspace) throw new Error(t("workspaces.errors.missingLayout"));
      if (dto.address || dto.port !== 0) throw invalid();
      return { ...metadata, kind: "workspace", workspace: dto.workspace };
    default:
      throw invalid();
  }
}

export function hostInventoryFromDto(dto: HostInventoryDto): HostInventory {
  return { hosts: dto.hosts.map(hostProfileFromDto), groups: dto.groups };
}

/** 旧 IPC 的空 SSH 字段集中在兼容边界构造，编辑器只提交对应协议配置。 */
export function hostDraftToDto(draft: HostDraft): HostDraftDto {
  const { id, name, groupId, tags, icon } = draft;
  const metadata = { id, name, groupId, tags, icon };
  switch (draft.kind) {
    case "ssh":
      return {
        ...metadata,
        kind: "ssh",
        address: draft.address,
        port: draft.port,
        username: draft.username,
        authPlan: draft.authPlan,
        startup: draft.startup,
      };
    case "telnet":
      return {
        ...metadata,
        kind: "telnet",
        address: draft.address,
        port: draft.port,
        username: "",
        authPlan: [],
      };
    case "serial":
      return {
        ...metadata,
        kind: "serial",
        serialConfig: draft.serialConfig,
        address: "",
        port: 0,
        username: "",
        authPlan: [],
      };
  }
}

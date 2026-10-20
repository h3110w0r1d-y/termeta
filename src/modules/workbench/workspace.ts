import { t } from "../../i18n";
import type {
  HostProfile,
  SavedLayout,
  SavedWorkspace,
} from "../../services/desktop/types";
import { terminalIds, type LayoutNode } from "./layout";
import type { TerminalItem, TerminalTabGroup } from "./types";

/** 整份校验先于会话创建，批量打开不能留下部分恢复的终端。 */
export function validateSavedWorkspace(
  host: HostProfile,
  profiles: readonly HostProfile[],
): SavedWorkspace {
  if (!host.workspace) throw new Error(t("workspaces.errors.missingLayout"));
  const connectionIds = new Set(
    profiles
      .filter((profile) => profile.kind !== "workspace")
      .map((profile) => profile.id),
  );
  let count = 0;
  const visit = (node: SavedLayout, depth: number) => {
    if (depth > 32 || count >= 64)
      throw new Error(t("workspaces.errors.layoutLimit"));
    if (node.kind === "connection") {
      if (!connectionIds.has(node.profileId))
        throw new Error(
          t("workspaces.errors.deletedConnection", { name: host.name }),
        );
      count++;
    } else {
      if (
        !["horizontal", "vertical"].includes(node.axis) ||
        !Number.isFinite(node.ratio) ||
        node.ratio < 0.1 ||
        node.ratio > 0.9
      )
        throw new Error(t("workspaces.errors.invalidSplit"));
      visit(node.first, depth + 1);
      visit(node.second, depth + 1);
    }
  };
  visit(host.workspace.root, 0);
  if (
    !Number.isInteger(host.workspace.focusedIndex) ||
    host.workspace.focusedIndex < 0 ||
    host.workspace.focusedIndex >= count
  )
    throw new Error(t("workspaces.errors.invalidFocus"));
  return host.workspace;
}

/** 每个叶子独立创建终端；重复引用同一连接也不复用会话。 */
export function restoreWorkspaceLayout(
  node: SavedLayout,
  createTerminal: (profileId: string) => string,
): LayoutNode {
  if (node.kind === "connection")
    return { kind: "terminal", terminalId: createTerminal(node.profileId) };
  return {
    kind: "split",
    id: crypto.randomUUID(),
    axis: node.axis,
    ratio: node.ratio,
    first: restoreWorkspaceLayout(node.first, createTerminal),
    second: restoreWorkspaceLayout(node.second, createTerminal),
  };
}

/** 存档仅投影已保存连接的引用，排除终端运行状态和临时连接。 */
export function captureWorkspace(
  group: TerminalTabGroup,
  terminals: readonly TerminalItem[],
  profiles: readonly HostProfile[],
): SavedWorkspace {
  const terminalById = new Map(
    terminals.map((terminal) => [terminal.terminalId, terminal]),
  );
  const connectionIds = new Set(
    profiles
      .filter((profile) => profile.kind !== "workspace")
      .map((profile) => profile.id),
  );
  const capture = (node: LayoutNode): SavedLayout => {
    if (node.kind === "terminal") {
      const target = terminalById.get(node.terminalId)?.target;
      if (target?.source !== "saved" || !connectionIds.has(target.profileId))
        throw new Error(t("workspaces.errors.unsavedConnections"));
      return { kind: "connection", profileId: target.profileId };
    }
    return {
      kind: "split",
      axis: node.axis,
      ratio: node.ratio,
      first: capture(node.first),
      second: capture(node.second),
    };
  };
  return {
    root: capture(group.root),
    focusedIndex: terminalIds(group.root).indexOf(group.focusedTerminalId),
  };
}

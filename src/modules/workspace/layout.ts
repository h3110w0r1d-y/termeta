export type SplitEdge = "left" | "right" | "top" | "bottom";
export type SplitAxis = "horizontal" | "vertical";

export type LayoutNode =
  | { kind: "terminal"; terminalId: string }
  | {
      kind: "split";
      id: string;
      axis: SplitAxis;
      ratio: number;
      first: LayoutNode;
      second: LayoutNode;
    };

// 所有矩形均相对整个终端舞台，单位为百分比，不持有 DOM 或像素尺寸。
export interface LayoutRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export interface SplitDivider {
  id: string;
  axis: SplitAxis;
  ratio: number;
  bounds: LayoutRect;
  position: number;
}

export function terminalIds(node: LayoutNode): string[] {
  return node.kind === "terminal"
    ? [node.terminalId]
    : [...terminalIds(node.first), ...terminalIds(node.second)];
}

export function removeTerminal(
  node: LayoutNode,
  terminalId: string,
): LayoutNode | undefined {
  if (node.kind === "terminal")
    return node.terminalId === terminalId ? undefined : node;

  const first = removeTerminal(node.first, terminalId);
  const second = removeTerminal(node.second, terminalId);
  // 移走叶子后直接提升剩余子树，避免保留没有终端的空分屏。
  if (!first) return second;
  if (!second) return first;
  return first === node.first && second === node.second
    ? node
    : { ...node, first, second };
}

export function splitTerminal(
  node: LayoutNode,
  targetId: string,
  incoming: LayoutNode,
  edge: SplitEdge,
): LayoutNode {
  if (node.kind === "terminal") {
    if (node.terminalId !== targetId) return node;
    const incomingFirst = edge === "left" || edge === "top";
    return {
      kind: "split",
      id: crypto.randomUUID(),
      axis: edge === "left" || edge === "right" ? "horizontal" : "vertical",
      ratio: 0.5,
      first: incomingFirst ? incoming : node,
      second: incomingFirst ? node : incoming,
    };
  }
  const first = splitTerminal(node.first, targetId, incoming, edge);
  const second = splitTerminal(node.second, targetId, incoming, edge);
  return first === node.first && second === node.second
    ? node
    : { ...node, first, second };
}

function clampRatio(ratio: number): number {
  return Number.isFinite(ratio) ? Math.min(0.9, Math.max(0.1, ratio)) : 0.5;
}

export function setSplitRatio(
  node: LayoutNode,
  splitId: string,
  ratio: number,
): LayoutNode {
  if (node.kind === "terminal") return node;
  if (node.id === splitId) {
    const nextRatio = clampRatio(ratio);
    return nextRatio === node.ratio ? node : { ...node, ratio: nextRatio };
  }
  const first = setSplitRatio(node.first, splitId, ratio);
  const second = setSplitRatio(node.second, splitId, ratio);
  return first === node.first && second === node.second
    ? node
    : { ...node, first, second };
}

export function layoutGeometry(node: LayoutNode): {
  panes: Map<string, LayoutRect>;
  dividers: SplitDivider[];
} {
  const panes = new Map<string, LayoutRect>();
  const dividers: SplitDivider[] = [];

  function visit(current: LayoutNode, bounds: LayoutRect) {
    if (current.kind === "terminal") {
      panes.set(current.terminalId, bounds);
      return;
    }
    const ratio = clampRatio(current.ratio);
    const horizontal = current.axis === "horizontal";
    const position = horizontal
      ? bounds.left + bounds.width * ratio
      : bounds.top + bounds.height * ratio;
    dividers.push({
      id: current.id,
      axis: current.axis,
      ratio,
      bounds,
      position,
    });
    // 子矩形沿用同一舞台坐标系，使嵌套移动无需重新挂载终端宿主。
    visit(current.first, {
      ...bounds,
      width: horizontal ? bounds.width * ratio : bounds.width,
      height: horizontal ? bounds.height : bounds.height * ratio,
    });
    visit(current.second, {
      left: horizontal ? position : bounds.left,
      top: horizontal ? bounds.top : position,
      width: horizontal ? bounds.width * (1 - ratio) : bounds.width,
      height: horizontal ? bounds.height : bounds.height * (1 - ratio),
    });
  }

  visit(node, { left: 0, top: 0, width: 100, height: 100 });
  return { panes, dividers };
}

export function edgeAtPoint(
  x: number,
  y: number,
  width: number,
  height: number,
): SplitEdge {
  // 先归一化，宽窄不同的子终端仍使用相同四向区域；零尺寸回退中心。
  const normalizedX =
    Number.isFinite(x) && Number.isFinite(width) && width > 0
      ? Math.min(1, Math.max(0, x / width))
      : 0.5;
  const normalizedY =
    Number.isFinite(y) && Number.isFinite(height) && height > 0
      ? Math.min(1, Math.max(0, y / height))
      : 0.5;
  const distances: [SplitEdge, number][] = [
    ["left", normalizedX],
    ["right", 1 - normalizedX],
    ["top", normalizedY],
    ["bottom", 1 - normalizedY],
  ];
  return distances.reduce((nearest, candidate) =>
    candidate[1] < nearest[1] ? candidate : nearest,
  )[0];
}

/** 原子计算窗格重排后的树；无效目标不移除来源，同树移动先收缩再分割。 */
export function relocateTerminal(
  source: LayoutNode,
  target: LayoutNode,
  terminalId: string,
  targetId: string,
  edge: SplitEdge,
): { sourceRoot: LayoutNode | undefined; targetRoot: LayoutNode } | undefined {
  if (
    terminalId === targetId ||
    !terminalIds(source).includes(terminalId) ||
    !terminalIds(target).includes(targetId) ||
    (source !== target && terminalIds(target).includes(terminalId))
  )
    return;
  const remainder = removeTerminal(source, terminalId);
  const destination = source === target ? remainder : target;
  if (!destination) return;
  const targetRoot = splitTerminal(
    destination,
    targetId,
    { kind: "terminal", terminalId },
    edge,
  );
  return { sourceRoot: source === target ? targetRoot : remainder, targetRoot };
}

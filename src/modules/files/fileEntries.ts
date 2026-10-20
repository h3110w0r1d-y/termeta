import type { FileEntry } from "@/services/desktop/types";

export const parentEntry: FileEntry = {
  name: "..",
  kind: "directory",
  linkTarget: null,
  targetKind: null,
  size: null,
  modified: null,
  permissions: null,
};
export function isDirectory(entry: FileEntry) {
  return (
    entry.kind === "directory" ||
    (entry.kind === "symlink" && entry.targetKind === "directory")
  );
}
export function entryLabel(entry: FileEntry) {
  return entry.kind === "symlink" && entry.linkTarget !== null
    ? `${entry.name} -> ${entry.linkTarget}`
    : entry.name;
}
export function directoryRows(
  entries: FileEntry[],
  hidden: boolean,
  order: "ascend" | "descend" | false,
) {
  // 过滤后排序，不修改原始分页结果；目录优先级不随排序方向变化。
  const files = entries.filter(
    (entry) => hidden || !entry.name.startsWith("."),
  );
  files.sort(
    (a, b) =>
      Number(isDirectory(b)) - Number(isDirectory(a)) ||
      (order
        ? (order === "descend" ? -1 : 1) * a.name.localeCompare(b.name)
        : 0),
  );
  // 上级目录不计入远端分页与选择。
  return [parentEntry, ...files];
}

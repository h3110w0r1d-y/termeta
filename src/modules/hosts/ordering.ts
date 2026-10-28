import type { HostMetadata } from "@/modules/hosts/model";
import type { HostSortMode } from "../settings/model";

// 主机浏览使用持久化手动顺序；尚未手动定位的新项按名称排在末尾。
export function compareHostOrder(
  a: Pick<HostMetadata, "id" | "name" | "sortOrder">,
  b: Pick<HostMetadata, "id" | "name" | "sortOrder">,
): number {
  return (
    a.sortOrder - b.sortOrder ||
    a.name.localeCompare(b.name) ||
    a.id.localeCompare(b.id)
  );
}

// 未使用项置后；时间相同按名称破平，使两种模式各自独立。
export function compareHostDisplayOrder(
  a: HostMetadata,
  b: HostMetadata,
  mode: HostSortMode,
): number {
  if (mode === "recent") {
    if (a.lastUsedAt === null && b.lastUsedAt !== null) return 1;
    if (a.lastUsedAt !== null && b.lastUsedAt === null) return -1;
    const elapsed = (b.lastUsedAt ?? 0) - (a.lastUsedAt ?? 0);
    if (elapsed) return elapsed;
    return a.name.localeCompare(b.name) || a.id.localeCompare(b.id);
  }
  return compareHostOrder(a, b);
}

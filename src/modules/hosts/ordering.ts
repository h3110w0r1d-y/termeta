import type { HostProfile } from "@/services/desktop/types";

// 主机浏览使用持久化手动顺序；尚未手动定位的新项按名称排在末尾。
export function compareHostOrder(a: HostProfile, b: HostProfile): number {
  return (
    a.sortOrder - b.sortOrder ||
    a.name.localeCompare(b.name) ||
    a.id.localeCompare(b.id)
  );
}

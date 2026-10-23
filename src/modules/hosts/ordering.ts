import type { HostMetadata } from "@/modules/hosts/model";

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

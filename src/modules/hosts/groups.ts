import { t } from "../../i18n";
import type { HostGroup } from "../../services/desktop/types";

// 分组关系只处理轻量元数据；遍历保留环保护，避免异常数据阻塞界面。
export function descendantIds(groups: HostGroup[], id: string): Set<string> {
  const ids = new Set([id]);
  const pending = [id];
  while (pending.length) {
    const parent = pending.pop();
    for (const group of groups) {
      if (group.parentId === parent && !ids.has(group.id)) {
        ids.add(group.id);
        pending.push(group.id);
      }
    }
  }
  return ids;
}

export function groupPath(groups: HostGroup[], id: string | null): string {
  const names: string[] = [];
  const seen = new Set<string>();
  while (id && !seen.has(id)) {
    seen.add(id);
    const group = groups.find((item) => item.id === id);
    if (!group) break;
    names.unshift(group.name);
    id = group.parentId;
  }
  return names.join(" / ") || t("未分组");
}

export function groupChoices(
  groups: HostGroup[],
  excluded = new Set<string>(),
) {
  return [...groups]
    .sort((a, b) =>
      groupPath(groups, a.id).localeCompare(groupPath(groups, b.id)),
    )
    .filter((group) => !excluded.has(group.id))
    .map((group) => ({ value: group.id, label: groupPath(groups, group.id) }));
}

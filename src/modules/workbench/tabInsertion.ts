export interface TabBounds {
  id: string;
  start: number;
  end: number;
}

export interface TabInsertion {
  beforeGroupId: string | null;
  position: number;
}

/** 相邻标签的两半与中间空隙共用一个插入槽，坐标沿当前布局主轴传入。 */
export function tabInsertionAt(
  tabs: readonly TabBounds[],
  point: number,
  sourceId?: string,
): TabInsertion | undefined {
  if (!tabs.length) return;
  const next = tabs.findIndex((tab) => point < (tab.start + tab.end) / 2);
  const index = next < 0 ? tabs.length : next;
  const previous = tabs[index - 1];
  const following = tabs[index];
  // 源标签仍在原位占位，但提交目标必须指向移除源标签后的右侧邻居。
  const beforeGroupId =
    (following?.id === sourceId ? tabs[index + 1]?.id : following?.id) ?? null;
  if (sourceId) {
    const sourceIndex = tabs.findIndex((tab) => tab.id === sourceId);
    if (sourceIndex < 0) return;
    if (beforeGroupId === (tabs[sourceIndex + 1]?.id ?? null)) return;
  }
  const position = previous
    ? following
      ? (previous.end + following.start) / 2
      : previous.end
    : following!.start;
  return { beforeGroupId, position };
}

/** 先移除来源再定位目标；无效或原位插入保留原数组及所有标签对象。 */
export function moveTabBefore<T extends { id: string }>(
  tabs: T[],
  sourceId: string,
  beforeGroupId: string | null,
): T[] {
  const source = tabs.find((tab) => tab.id === sourceId);
  if (!source || beforeGroupId === sourceId) return tabs;
  const remaining = tabs.filter((tab) => tab.id !== sourceId);
  const index =
    beforeGroupId === null
      ? remaining.length
      : remaining.findIndex((tab) => tab.id === beforeGroupId);
  if (index < 0) return tabs;
  remaining.splice(index, 0, source);
  return remaining.every((tab, at) => tab === tabs[at]) ? tabs : remaining;
}

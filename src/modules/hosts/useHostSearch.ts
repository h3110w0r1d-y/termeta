import { computed, type Ref } from "vue";
import { useHosts } from "./store";
import { useSettings } from "../settings/store";
import { groupPath } from "./groups";
import { matchesText, searchWords, toSearchableName } from "./search";

// 保险箱下拉和新建标签页弹窗共用索引、字段偏好及收藏排序。
export function useHostSearch(query: Ref<string>, includeAllWhenEmpty = false) {
  const store = useHosts();
  const settings = useSettings();
  // 索引只随轻量主机元数据更新，不缓存输入历史，也不影响下方分组列表。
  const index = computed(() =>
    store.hosts.map((host) => ({
      host,
      fields: settings.value.hostSearch.fields.flatMap((field) => {
        const values = {
          name: [host.name],
          address: [host.address],
          username: [host.username],
          tags: host.tags,
          group: [groupPath(store.groups, host.groupId)],
        }[field];
        return values.map((value) => ({
          text: value.toLocaleLowerCase(),
          pinyin: settings.value.hostSearch.pinyin
            ? toSearchableName(value)
            : undefined,
        }));
      }),
    })),
  );
  const matches = computed(() => {
    const words = query.value
      .trim()
      .split(/\s+/)
      .filter(Boolean)
      .map((word) => ({
        text: word.toLocaleLowerCase(),
        pinyin: settings.value.hostSearch.pinyin
          ? searchWords(word)
          : undefined,
      }));
    if (!words.length && !includeAllWhenEmpty) return [];
    return index.value
      .filter((entry) =>
        words.every((word) =>
          entry.fields.some((field) =>
            field.pinyin && word.pinyin
              ? matchesText(field.pinyin, word.pinyin)
              : field.text.includes(word.text),
          ),
        ),
      )
      .map((entry) => entry.host)
      .sort(
        (a, b) =>
          Number(b.favorite) - Number(a.favorite) ||
          a.name.localeCompare(b.name),
      );
  });
  return matches;
}

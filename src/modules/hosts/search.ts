import { PINYIN_CONF } from "../../assets/conf/PINYIN_CONF.ts";
// 匹配算法改编自 roco-helper-web 的 petNameSearch，仅保留文本与拼音匹配。
function uniqueSorted(values: string[]) {
  return Array.from(new Set(values)).sort((a, b) =>
    a.localeCompare(b, "zh-CN"),
  );
}

interface PinyinNamePart {
  text: string;
  pinyins: string[];
  initials: string[];
  matches: string[];
}

interface SearchableName {
  normalizedName: string;
  parts: PinyinNamePart[];
  initials: string;
}

export function searchWords(query: string) {
  const words = toSearchWords(query);
  const pinyinWords = words.map(toPinyinSearchText);
  return { words, pinyinWords, changed: hasChangedWords(words, pinyinWords) };
}

export function matchesText(
  search: SearchableName,
  query: ReturnType<typeof searchWords>,
) {
  return (
    matchesSearchWords(search, query.words) ||
    (query.changed && matchesSearchWords(search, query.pinyinWords))
  );
}

export function toSearchableName(name: string): SearchableName {
  const parts = Array.from(name).map((char) => {
    const text = char.toLocaleLowerCase();
    const pinyinCandidates = pinyinForChar(char);
    const fullPinyins = sortByLongestFirst(
      pinyinCandidates.filter((item) => item.length > 1),
    );
    const pinyins = fullPinyins.length > 0 ? fullPinyins : pinyinCandidates;
    const initials = uniqueSorted(
      [
        ...pinyinCandidates.filter((item) => item.length === 1),
        ...pinyins.map((item) => item.charAt(0)),
      ].filter(Boolean),
    );
    const matches = uniqueSorted(
      [text, ...pinyins, ...initials].filter(Boolean),
    );

    return { text, pinyins, initials, matches };
  });
  const search = {
    normalizedName: name.toLocaleLowerCase(),
    parts,
    initials: parts.map(bestInitial).join(""),
  };

  return search;
}

function pinyinForChar(char: string) {
  const codePoint = char.codePointAt(0);

  if (codePoint === undefined) {
    return [];
  }

  const hex = codePoint.toString(16).toUpperCase();

  return uniqueSorted(
    (PINYIN_CONF[hex] ?? [])
      .map((item) => item.trim().toLocaleLowerCase())
      .filter((item) => /^[a-z]+$/.test(item)),
  );
}

function toSearchWords(text: string) {
  return text.toLocaleLowerCase().split(/\s+/).filter(Boolean);
}

function toPinyinSearchText(text: string) {
  return Array.from(text.toLocaleLowerCase())
    .map((char) => longestPinyinForChar(char) ?? char)
    .join("");
}

function longestPinyinForChar(char: string) {
  return sortByLongestFirst(pinyinForChar(char))[0];
}

function sortByLongestFirst(values: string[]) {
  return [...values].sort(
    (left, right) => right.length - left.length || left.localeCompare(right),
  );
}

function bestInitial(part: PinyinNamePart) {
  return part.initials[0] ?? part.text;
}

function matchesSearchableName(search: SearchableName, query: string) {
  const normalizedQuery = query.trim().toLocaleLowerCase();

  if (!normalizedQuery) {
    return true;
  }

  if (
    search.normalizedName.includes(normalizedQuery) ||
    search.initials.includes(normalizedQuery)
  ) {
    return true;
  }

  return search.parts.some((_, index) =>
    matchesNameFrom(search.parts, normalizedQuery, index, 0),
  );
}

function matchesSearchWords(search: SearchableName, words: string[]) {
  return words.every((word) => matchesSearchableName(search, word));
}

function hasChangedWords(words: string[], changedWords: string[]) {
  return words.some((word, index) => word !== changedWords[index]);
}

function matchesNameFrom(
  parts: PinyinNamePart[],
  query: string,
  partIndex: number,
  queryIndex: number,
  memo = new Set<string>(),
): boolean {
  if (queryIndex >= query.length) {
    return true;
  }

  if (partIndex >= parts.length) {
    return false;
  }

  const memoKey = `${partIndex}:${queryIndex}`;

  if (memo.has(memoKey)) {
    return false;
  }

  memo.add(memoKey);

  for (const match of parts[partIndex]?.matches ?? []) {
    if (query.startsWith(match, queryIndex)) {
      if (
        matchesNameFrom(
          parts,
          query,
          partIndex + 1,
          queryIndex + match.length,
          memo,
        )
      ) {
        return true;
      }
    } else if (match.startsWith(query.slice(queryIndex))) {
      return true;
    }
  }

  return false;
}

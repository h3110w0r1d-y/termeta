const punctuationMap: Readonly<Record<string, string>> = {
  "！": "!",
  "￥": "$",
  "（": "(",
  "）": ")",
  "《": "<",
  "》": ">",
  "「": "{",
  "」": "}",
  "【": "[",
  "】": "]",
  "？": "?",
  "：": ":",
  "；": ";",
  "“": '"',
  "”": '"',
  "，": ",",
  "。": ".",
  "、": "/",
  "‘": "'",
  "’": "'",
};

/** 只替换约定的标点，保留中文正文及其他全角字符。 */
export function convertChinesePunctuation(data: string) {
  return data.replace(
    /[！￥（）《》「」【】？：；“”，。、‘’]/g,
    (character) => punctuationMap[character] ?? character,
  );
}

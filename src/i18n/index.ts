import { computed, ref, watch } from "vue";
import {
  zhCN,
  zhTW,
  enUS,
  jaJP,
  koKR,
  dateZhCN,
  dateZhTW,
  dateEnUS,
  dateJaJP,
  dateKoKR,
} from "naive-ui";
import { messages } from "./messages";
import type { MessageKey, TranslationArguments } from "./messages";

export type { MessageKey, PlainMessageKey } from "./messages";

export type Locale = keyof typeof messages;
export type Language = "system" | Locale;
const componentLocales = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  "en-US": enUS,
  "ja-JP": jaJP,
  "ko-KR": koKR,
};
const dateLocales = {
  "zh-CN": dateZhCN,
  "zh-TW": dateZhTW,
  "en-US": dateEnUS,
  "ja-JP": dateJaJP,
  "ko-KR": dateKoKR,
};

// 先判断显式字形，再判断地区；繁体地区与通用日/韩语言映射到支持的词典。
export function resolveSystemLocale(languages: readonly string[]): Locale {
  for (const language of languages) {
    const [base, ...subtags] = language.toLowerCase().split("-");
    if (base === "zh") {
      if (subtags.includes("hant")) return "zh-TW";
      if (subtags.includes("hans")) return "zh-CN";
      return subtags.some((part) => ["tw", "hk", "mo"].includes(part))
        ? "zh-TW"
        : "zh-CN";
    }
    if (base === "en") return "en-US";
    if (base === "ja") return "ja-JP";
    if (base === "ko") return "ko-KR";
  }
  return "en-US";
}
const preference = ref<Language>("system");
const systemLanguages = ref([...navigator.languages]);
export const locale = computed<Locale>(() => {
  if (preference.value !== "system") return preference.value;
  return resolveSystemLocale(systemLanguages.value);
});
export const naiveLocale = computed(() => componentLocales[locale.value]);
export const naiveDateLocale = computed(() => dateLocales[locale.value]);
export function normalizeLanguage(value: unknown): Language {
  return typeof value === "string" &&
    Object.prototype.hasOwnProperty.call(messages, value)
    ? (value as Locale)
    : "system";
}
export function setLanguage(value: Language) {
  preference.value = normalizeLanguage(value);
}
// key 与译文分离；类型约束词条和命名参数，插值不解析 HTML 或执行表达式。
export function t<Key extends MessageKey>(
  key: Key,
  ...args: TranslationArguments<Key>
): string {
  const parameters: Record<string, string | number> = args[0] ?? {};
  const translated = messages[locale.value][key];
  return translated.replace(/\{(\w+)\}/g, (placeholder, name: string) =>
    Object.prototype.hasOwnProperty.call(parameters, name)
      ? String(parameters[name])
      : placeholder,
  );
}
export function formatDate(
  value: string | number | Date,
  options?: Intl.DateTimeFormatOptions,
) {
  const date = value instanceof Date ? value : new Date(value);
  return Number.isNaN(date.getTime())
    ? String(value)
    : date.toLocaleString(locale.value, options);
}
// 两个窗口各自监听系统语言；加密设置读回后通过同一入口覆盖偏好。
export function initializeLocale() {
  watch(
    locale,
    (value) => {
      document.documentElement.lang = value;
    },
    { immediate: true },
  );
  window.addEventListener("languagechange", () => {
    systemLanguages.value = [...navigator.languages];
  });
}

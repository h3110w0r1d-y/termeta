import { computed, ref, watch } from "vue";
import { zhCN, enUS, dateZhCN, dateEnUS } from "naive-ui";
import { messages } from "./messages";
import type { MessageKey, TranslationArguments } from "./messages";

export type { MessageKey, PlainMessageKey } from "./messages";

export type Language = "system" | "zh-CN" | "en-US";
export type Locale = Exclude<Language, "system">;
const preference = ref<Language>("system");
const systemLanguages = ref([...navigator.languages]);
export const locale = computed<Locale>(() => {
  if (preference.value !== "system") return preference.value;
  for (const language of systemLanguages.value) {
    if (/^zh(?:-|$)/i.test(language)) return "zh-CN";
    if (/^en(?:-|$)/i.test(language)) return "en-US";
  }
  return "en-US";
});
export const naiveLocale = computed(() =>
  locale.value === "zh-CN" ? zhCN : enUS,
);
export const naiveDateLocale = computed(() =>
  locale.value === "zh-CN" ? dateZhCN : dateEnUS,
);
export function normalizeLanguage(value: unknown): Language {
  return value === "zh-CN" || value === "en-US" ? value : "system";
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

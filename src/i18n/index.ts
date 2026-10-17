import { computed, ref, watch } from "vue";
import { zhCN, enUS, dateZhCN, dateEnUS } from "naive-ui";
import common from "./messages/common";
import hosts from "./messages/hosts";
import terminal from "./messages/terminal";
import settings from "./messages/settings";

export type Language = "system" | "zh-CN" | "en-US";
export type Locale = Exclude<Language, "system">;
const preference = ref<Language>("system");
const systemLanguages = ref([...navigator.languages]);
const english: Record<string, string> = Object.assign(Object.create(null), {
  ...common,
  ...hosts,
  ...terminal,
  ...settings,
});
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
// 中文原文兼作词条标识；插值只替换命名参数，不解析 HTML 或执行表达式。
export function t(
  message: string,
  parameters: Record<string, string | number> = {},
): string {
  const translated =
    locale.value === "en-US" ? (english[message] ?? message) : message;
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

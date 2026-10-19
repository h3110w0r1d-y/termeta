import { computed, watch } from "vue";
import { darkTheme, type GlobalThemeOverrides } from "naive-ui";
import { useSettings } from "../modules/settings/store";

export function useAppAppearance() {
  const settings = useSettings();
  const naiveTheme = darkTheme;
  const primaryColor = computed(
    () => settings.value.primaryColor || darkTheme.common.primaryColor,
  );
  const naiveThemeOverrides = computed<GlobalThemeOverrides>(() => {
    if (!settings.value.primaryColor) return {};
    const color = primaryColor.value;
    // 从主色分别混入白色和黑色，为库控件派生悬停和按下状态。
    const mix = (target: number, ratio: number) =>
      "#" +
      [1, 3, 5]
        .map((offset) => {
          const channel = parseInt(color.slice(offset, offset + 2), 16);
          return Math.round(channel + (target - channel) * ratio)
            .toString(16)
            .padStart(2, "0");
        })
        .join("");
    return {
      common: {
        primaryColor: color,
        primaryColorHover: mix(255, 0.16),
        primaryColorPressed: mix(0, 0.16),
        primaryColorSuppl: color,
      },
    };
  });
  watch(
    primaryColor,
    (color) => {
      document.documentElement.style.setProperty("--accent", color);
    },
    { immediate: true },
  );
  return { naiveTheme, naiveThemeOverrides };
}

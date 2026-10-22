import { computed, watch } from "vue";
import { darkTheme, type GlobalThemeOverrides } from "naive-ui";
import { useSettings } from "@/modules/settings/store";
import { mixColors } from "@/theme/colors";

export function useAppAppearance() {
  const settings = useSettings();
  const naiveTheme = darkTheme;
  const primaryColor = computed(
    () => settings.value.primaryColor || darkTheme.common.primaryColor,
  );
  const naiveThemeOverrides = computed<GlobalThemeOverrides>(() => {
    // 根主题统一圆角，局部终端配色 Provider 通过库的主题合并继续继承。
    const common = { borderRadius: "6px", borderRadiusSmall: "4px" };
    if (!settings.value.primaryColor) return { common };
    const color = primaryColor.value;
    // 从主色分别混入白色和黑色，与局部表面共用颜色混合规则。
    return {
      common: {
        ...common,
        primaryColor: color,
        primaryColorHover: mixColors(color, "#ffffff", 84),
        primaryColorPressed: mixColors(color, "#000000", 84),
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

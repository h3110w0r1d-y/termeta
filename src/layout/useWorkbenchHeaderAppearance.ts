import { computed } from "vue";
import {
  darkTheme,
  lightTheme,
  useThemeVars,
  type GlobalThemeOverrides,
} from "naive-ui";
import { useSettings } from "@/modules/settings/store";
import { useWorkbenchStore } from "@/modules/workbench/store";
import { resolveTerminalScheme } from "@/theme/schemes";
import { colorOverlay as overlay, isLightBackground } from "@/theme/colors";

/** 窗口标题栏与应用标签共用配色；局部主题不扩展到工作台内容和终端宿主。 */
export function useWorkbenchHeaderAppearance() {
  const themeVars = useThemeVars();
  const settings = useSettings();
  const workbench = useWorkbenchStore();
  const headerColors = computed(() => {
    const terminal = workbench.activeTerminal;
    return terminal
      ? resolveTerminalScheme(terminal, settings.value).colors
      : undefined;
  });
  const headerBackground = computed(
    () => headerColors.value?.background ?? "var(--surface)",
  );
  const headerLight = computed(() => {
    const background = headerColors.value?.background;
    if (!background) return false;
    return isLightBackground(background);
  });
  const theme = computed(() => (headerLight.value ? lightTheme : darkTheme));
  const headerForeground = computed(
    () => headerColors.value?.foreground ?? themeVars.value.textColor1,
  );
  const headerUi = computed(() =>
    headerColors.value
      ? headerColors.value.ui || headerForeground.value
      : themeVars.value.textColor3,
  );
  const themeOverrides = computed<GlobalThemeOverrides>(() => {
    if (!headerColors.value) return {};
    // 公开主题覆盖使用局部变量，按钮悬停时切到前景色，菜单保持库的明暗表面。
    const color = "var(--app-header-button-color)";
    return {
      Button: {
        textColor: color,
        textColorHover: color,
        textColorPressed: color,
        textColorFocus: color,
        colorQuaternaryHover: overlay(headerUi.value, 20),
        colorQuaternaryPressed: overlay(headerUi.value, 20),
      },
    };
  });
  const style = computed(() => {
    const colors = headerColors.value;
    const ui = headerUi.value;
    const foreground = headerForeground.value;
    const monochrome = headerLight.value ? "#000000" : "#ffffff";
    return {
      "--text": foreground,
      "--muted": ui,
      "--app-header-button-color": ui,
      "--app-tab-background": overlay(
        colors ? ui : monochrome,
        colors ? 10 : 5,
      ),
      "--app-tab-hover": overlay(colors ? ui : monochrome, colors ? 20 : 10),
      "--app-tab-active": overlay(
        colors ? foreground : monochrome,
        colors ? 20 : 10,
      ),
      "--app-tab-drag": overlay(colors ? ui : monochrome, colors ? 30 : 20),
      "--app-tab-text": foreground,
      "--app-tab-close-hover": colors
        ? overlay(ui, 10)
        : themeVars.value.closeColorHover,
      "--app-tab-close-selected-hover": colors
        ? overlay(foreground, 10)
        : themeVars.value.closeColorHover,
      "--app-tab-radius": themeVars.value.borderRadius,
      "--app-tabs-background": headerBackground.value,
      "--app-tabs-border": colors ? "transparent" : undefined,
      color: ui,
      colorScheme: headerLight.value ? "light" : "dark",
    };
  });
  return { theme, themeOverrides, style };
}

import {
  computed,
  inject,
  provide,
  type CSSProperties,
  type InjectionKey,
  type ComputedRef,
} from "vue";
import { darkTheme, lightTheme, type GlobalThemeOverrides } from "naive-ui";
import type { Scheme } from "../settings/model";
import { colorOverlay, isLightBackground, mixColors } from "./appearance";

const surfaceStyleKey: InjectionKey<ComputedRef<CSSProperties>> = Symbol(
  "terminal-surface-style",
);

// 弹层保留 Vue 注入关系，但传送到 body 后须显式携带局部 CSS 变量。
export function useTerminalSurfaceStyle() {
  return inject(surfaceStyleKey, undefined);
}

export function useTerminalSurface(colors: () => Scheme["colors"]) {
  const palette = computed(() => {
    const source = colors();
    const { background, foreground } = source;
    const ui = source.ui || foreground;
    const light = isLightBackground(background);
    const mix = (color: string, percent: number) =>
      mixColors(color, background, percent);
    return {
      background,
      foreground,
      ui,
      light,
      surface: mix(ui, 4),
      hover: mix(ui, 12),
      pressed: mix(ui, 18),
      border: colorOverlay(ui, 20),
      muted: mix(foreground, 72),
      disabled: mix(ui, 35),
    };
  });
  const theme = computed(() => (palette.value.light ? lightTheme : darkTheme));
  const themeOverrides = computed<GlobalThemeOverrides>(() => {
    const p = palette.value;
    return {
      common: {
        baseColor: p.background,
        bodyColor: p.background,
        cardColor: p.surface,
        modalColor: p.surface,
        popoverColor: p.surface,
        tableColor: p.background,
        tableHeaderColor: p.surface,
        tableColorHover: p.hover,
        tableColorStriped: p.surface,
        inputColor: p.background,
        inputColorDisabled: p.surface,
        // 普通文字与顶部未激活标签共用 UI 色，终端前景色只用于交互强调。
        textColorBase: p.ui,
        textColor1: p.ui,
        textColor2: p.ui,
        textColor3: p.ui,
        textColorDisabled: p.disabled,
        placeholderColor: p.ui,
        placeholderColorDisabled: p.disabled,
        iconColor: p.ui,
        iconColorHover: p.foreground,
        iconColorPressed: p.foreground,
        iconColorDisabled: p.disabled,
        borderColor: p.border,
        dividerColor: p.border,
        hoverColor: p.hover,
        pressedColor: p.pressed,
        buttonColor2: p.surface,
        buttonColor2Hover: p.hover,
        buttonColor2Pressed: p.pressed,
        closeIconColor: p.ui,
        closeIconColorHover: p.foreground,
        closeIconColorPressed: p.foreground,
        closeColorHover: p.hover,
        closeColorPressed: p.pressed,
        clearColor: p.ui,
        clearColorHover: p.foreground,
        clearColorPressed: p.foreground,
        scrollbarColor: colorOverlay(p.ui, 30),
        scrollbarColorHover: colorOverlay(p.ui, 50),
        progressRailColor: p.hover,
        railColor: p.hover,
        primaryColor: p.foreground,
        primaryColorHover: p.muted,
        primaryColorPressed: p.foreground,
        primaryColorSuppl: p.foreground,
      },
      // 深色主题的输入默认边框为透明，须显式沿用卡片的普通边框色。
      Input: {
        border: `1px solid ${p.border}`,
      },
      Select: {
        peers: {
          InternalSelection: {
            border: `1px solid ${p.border}`,
          },
        },
      },
      Button: {
        colorHover: mixColors(p.ui, p.background, 12),
        colorPressed: mixColors(p.ui, p.background, 18),
        textColorHover: p.foreground,
        textColorPressed: p.foreground,
        borderHover: `1px solid ${colorOverlay(p.ui, 60)}`,
        borderPressed: `1px solid ${colorOverlay(p.ui, 75)}`,
        colorQuaternaryHover: mixColors(p.ui, p.background, 24),
        colorQuaternaryPressed: mixColors(p.ui, p.background, 32),
      },
    };
  });
  const style = computed<CSSProperties>(() => {
    const p = palette.value;
    return {
      "--bg": p.background,
      "--surface": p.background,
      "--surface-hover": p.hover,
      "--border": p.border,
      "--text": p.ui,
      "--muted": p.ui,
      "--accent": p.foreground,
      "--danger": theme.value.common!.errorColor,
      "--input": p.surface,
      color: p.ui,
      colorScheme: p.light ? "light" : "dark",
    };
  });
  provide(surfaceStyleKey, style);
  return { theme, themeOverrides, style };
}

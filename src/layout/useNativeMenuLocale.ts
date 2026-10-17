import { onBeforeUnmount, ref, watch } from "vue";
import { locale } from "../i18n";
import { isMac } from "../services/commands/registry";
import { desktop } from "../services/desktop/client";

// 仅根主窗口同步应用级菜单，设置窗口启动时的系统语言不能覆盖已保存偏好。
export function useNativeMenuLocale() {
  const error = ref(false);
  let pending = Promise.resolve();
  let disposed = false;
  function synchronize() {
    if (!isMac) return Promise.resolve();
    const requested = locale.value;
    pending = pending.then(async () => {
      if (disposed || requested !== locale.value) return;
      try {
        await desktop.setNativeMenuLocale(requested);
        if (!disposed && requested === locale.value) error.value = false;
      } catch {
        if (!disposed && requested === locale.value) error.value = true;
      }
    });
    return pending;
  }
  watch(locale, synchronize, { immediate: true });
  onBeforeUnmount(() => {
    disposed = true;
  });
  return { error, retry: synchronize };
}

import { onBeforeUnmount, ref } from "vue";
import { desktop, errorMessage } from "@/services/desktop/client";
import { useSettings } from "./store";

// 先订阅再读取，并串行重读，避免初始化与通知交错导致旧值回写。
export function useSettingsSync() {
  const store = useSettings();
  const error = ref("");
  let stop: (() => void) | undefined;
  let disposed = false;
  let pending = Promise.resolve();
  function reload() {
    pending = pending
      .catch(() => {})
      .then(async () => {
        try {
          await store.load();
          error.value = "";
        } catch (e) {
          error.value = errorMessage(e);
          throw e;
        }
      });
    return pending;
  }
  async function start() {
    const unlisten = await desktop.onSettingsChanged(() => {
      void reload().catch(() => {});
    });
    if (disposed) {
      unlisten();
      return;
    }
    stop = unlisten;
    await reload();
  }
  onBeforeUnmount(() => {
    disposed = true;
    stop?.();
  });
  return { start, error };
}

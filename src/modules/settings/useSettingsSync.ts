import { onBeforeUnmount, ref } from "vue";
import { desktop, errorMessage } from "@/services/desktop/client";
import { useSettings } from "./store";

// 先订阅再读取，并串行重读，避免初始化与通知交错导致旧值回写。
export function useSettingsSync() {
  const store = useSettings();
  const error = ref("");
  let stop: (() => void) | undefined;
  let subscribing: Promise<void> | undefined;
  let starting: Promise<void> | undefined;
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
  function start(): Promise<void> {
    if (disposed) return Promise.resolve();
    // 窗口订阅只建立一次；解锁初始化失败时只重试读取，不叠加监听。
    starting ??= (async () => {
      subscribing ??= desktop
        .onSettingsChanged(() => {
          void reload().catch(() => {});
        })
        .then((unlisten) => {
          if (disposed) unlisten();
          else stop = unlisten;
        })
        .catch((cause: unknown) => {
          subscribing = undefined;
          throw cause;
        });
      await subscribing;
      if (!disposed) await reload();
    })().finally(() => {
      starting = undefined;
    });
    return starting;
  }
  onBeforeUnmount(() => {
    disposed = true;
    stop?.();
  });
  return { start, error };
}
